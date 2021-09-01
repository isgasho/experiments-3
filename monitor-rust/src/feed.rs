use std::{
    collections::HashMap,
    io,
    io::{Error, ErrorKind},
    string::String,
    sync::{Arc, Mutex},
};

use tokio::{
    select,
    sync::mpsc,
    time::{self, Duration},
};

use serde::{Deserialize, Serialize};

use tungstenite::{connect, Message};
use url::Url;

use crate::block;

#[derive(Debug)]
pub struct Feeder {
    request_timeout: Duration,
    observer_url: String,

    stop_tx: mpsc::Sender<()>,

    last_ex_rates: Arc<Mutex<HashMap<String, f64>>>,
    last_supply: Arc<Mutex<HashMap<String, u64>>>,
}

impl Feeder {
    pub fn new(request_timeout: Duration, observer_url: String) -> (Self, mpsc::Receiver<()>) {
        let (stop_ch_tx, stop_ch_rx) = mpsc::channel(1);
        (
            Self {
                request_timeout: request_timeout,
                observer_url: observer_url,
                stop_tx: stop_ch_tx,
                last_ex_rates: Arc::new(Mutex::new(HashMap::new())),
                last_supply: Arc::new(Mutex::new(HashMap::new())),
            },
            stop_ch_rx,
        )
    }

    pub async fn poll(&self, mut stop_rx: mpsc::Receiver<()>) -> io::Result<()> {
        println!(
            "creating web socket connection to {:?} with timeout {}",
            self.request_timeout, self.observer_url
        );
        let u = match Url::parse("wss://observer.terra.dev") {
            Ok(u) => u,
            Err(e) => panic!("failed to parse observer URL ({})!", e),
        };
        let mut socket = match connect(u) {
            Ok((s, _)) => s,
            Err(e) => panic!("failed to connect ({})!", e),
        };
        println!("created web socket connection",);

        println!("polling");
        'outer: loop {
            // ref. https://docs.rs/tokio/1.11.0/tokio/macro.select.html#examples
            let sleep = time::sleep(Duration::from_millis(100));
            tokio::pin!(sleep);
            select! {
                _ = stop_rx.recv() => {
                    println!("received stop_rx");
                    break 'outer;
                }
                _ = &mut sleep, if !sleep.is_elapsed() => {},
            };

            // write to web socket
            let req = Request::new("new_block", "columbus-4");
            let txt = match encode_request(req) {
                Ok(v) => v,
                Err(e) => panic!("failed to encode request {}", e),
            };
            socket.write_message(Message::Text(txt)).unwrap();

            // read from web socket connection
            let m: Message;
            loop {
                match socket.read_message() {
                    Ok(v) => {
                        m = v;
                        break;
                    }
                    Err(e) => {
                        println!("failed to read message {}", e);
                    }
                };
                let sleep = time::sleep(Duration::from_millis(100));
                tokio::pin!(sleep);
                select! {
                    _ = stop_rx.recv() => {
                        println!("received stop_rx");
                        break 'outer;
                    }
                    _ = &mut sleep, if !sleep.is_elapsed() => {},
                };
            }
            // parse response from web socket
            let raw = m.into_data();
            let parsed = match block::parse(&raw) {
                Ok(v) => v,
                Err(e) => panic!("failed to parse message {}", e),
            };

            let rates = match parsed.get_exchange_rates() {
                Ok(v) => v,
                Err(e) => {
                    println!("failed get_exchange_rates {}", e);
                    HashMap::new()
                }
            };
            let supply = match parsed.get_supply() {
                Ok(v) => v,
                Err(e) => {
                    println!("failed get_supply {}", e);
                    HashMap::new()
                }
            };

            println!("updating");

            // TODO: find an easy way to overwrite hash map?

            // if exchange rate is found
            if rates.len() > 0 {
                let mut last_ex_rates = self
                    .last_ex_rates
                    .lock()
                    .map_err(|_| Error::new(ErrorKind::InvalidInput, "failed to acquire lock"))?;
                last_ex_rates.clear();
                for (k, v) in rates.iter() {
                    last_ex_rates.insert(k.to_string(), *v);
                }
            }

            // if supply is found
            if supply.len() > 0 {
                let mut last_supply = self
                    .last_supply
                    .lock()
                    .map_err(|_| Error::new(ErrorKind::InvalidInput, "failed to acquire lock"))?;
                last_supply.clear();
                for (k, v) in supply.iter() {
                    last_supply.insert(k.to_string(), *v);
                }
            }
        }

        match socket.close(None) {
            Ok(_) => {}
            Err(e) => println!("failed to close ({})!", e),
        };
        Ok(())
    }

    pub fn prices(&self) -> io::Result<block::Prices> {
        let last_ex_rates = self
            .last_ex_rates
            .lock()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "failed to acquire lock"))?;
        if last_ex_rates.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput, "no rates found"));
        }

        let last_supply = self
            .last_supply
            .lock()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "failed to acquire lock"))?;
        if last_supply.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput, "no supply found"));
        }

        let mut prices: Vec<block::Price> = Vec::new();
        for (denom, amount) in last_supply.iter() {
            let rate = match last_ex_rates.get(denom) {
                Some(&v) => v,
                _ => {
                    println!("denom {} rate not found", denom);
                    0.0
                }
            };
            prices.push(block::Price::new(denom.as_str(), rate, *amount));
        }
        Ok(block::Prices::new(prices))
    }

    pub async fn stop(&self) -> io::Result<()> {
        println!("stopping feeder");

        match self.stop_tx.send(()).await {
            Ok(()) => println!("sent stop_tx"),
            Err(e) => {
                println!("failed to send stop_tx: {}", e);
                return Err(Error::new(ErrorKind::Other, format!("failed to send")));
            }
        }

        println!("stopped feeder");
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Request {
    pub subscribe: String,
    pub chain_id: String,
}

impl Request {
    pub fn new(subscribe: &str, chain_id: &str) -> Self {
        Self {
            subscribe: String::from(subscribe),
            chain_id: String::from(chain_id),
        }
    }
}

pub fn encode_request(req: Request) -> io::Result<String> {
    let rs = serde_json::to_string(&req)?;
    Ok(String::from(rs))
}
