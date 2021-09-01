use std::{
    collections::HashMap,
    io,
    io::{Error, ErrorKind},
};

use serde::{Deserialize, Serialize};

pub fn parse(b: &[u8]) -> io::Result<Block> {
    serde_json::from_slice(b).map_err(|e| {
        return Error::new(ErrorKind::InvalidInput, format!("invalid JSON: {}", e));
    })
}

#[test]
fn test_parse() {
    let bb = b"
        {
            \"chain_id\": \"1\",
            \"type\": \"2\"
        }";
    let ret = parse(bb);
    assert!(ret.is_ok());
    let t = ret.unwrap();
    assert_eq!(t.chain_id, "1");
    assert_eq!(t.chain_type, "2");

    use std::fs;
    let data = fs::read("src/test-data/output-1.json").expect("failed to read file");
    let ret = parse(&data);
    assert!(ret.is_ok());
    let b1 = ret.unwrap();
    // println!("block: {:?}", b1);
    let ret = b1.get_exchange_rates();
    assert!(ret.is_ok());
    let rates = ret.unwrap();
    println!("exchange_rates: {:?}", rates);
    let ret = b1.get_supply();
    assert!(ret.is_ok());
    let supply = ret.unwrap();
    println!("supply: {:?}", supply);

    let data = fs::read("src/test-data/output-2.json").expect("failed to read file");
    let ret = parse(&data);
    assert!(ret.is_ok());
    let b2 = ret.unwrap();
    // println!("block: {:?}", b2);
    let ret = b2.get_exchange_rates();
    assert!(ret.is_ok());
    let rates = ret.unwrap();
    println!("exchange_rates: {:?}", rates);
    let ret = b2.get_supply();
    assert!(ret.is_ok());
    let supply = ret.unwrap();
    println!("supply: {:?}", supply);
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Block {
    pub chain_id: String,
    #[serde(default, rename = "type", skip_serializing_if = "String::is_empty")]
    pub chain_type: String,
    pub data: Option<Data>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Data {
    pub txs: Vec<Tx>,
    pub supply: Vec<Supply>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Tx {
    pub height: String,
    pub logs: Vec<Log>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Log {
    pub events: Vec<Event>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Event {
    #[serde(default, rename = "type", skip_serializing_if = "String::is_empty")]
    pub event_type: String,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Attribute {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Supply {
    pub denom: String,
    pub amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Prices {
    pub prices: Vec<Price>,
}

impl Prices {
    pub fn new(prices: Vec<Price>) -> Self {
        Self { prices: prices }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Price {
    pub denom: String,
    pub price: f64,
    pub volume: u64,
}

impl Price {
    pub fn new(denom: &str, price: f64, volume: u64) -> Self {
        Self {
            denom: denom.to_string(),
            price: price,
            volume: volume,
        }
    }
}

impl Block {
    pub fn get_exchange_rates(&self) -> io::Result<HashMap<String, f64>> {
        let data = match &self.data {
            Some(v) => v,
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("no data found for {}", self.chain_id),
                ))
            }
        };

        let mut ex_rates = String::new();
        'outer: for tx in data.txs.iter() {
            for lv in &tx.logs {
                for ev in &lv.events {
                    if ev.event_type != "aggregate_vote" {
                        continue;
                    }
                    for attr in &ev.attributes {
                        if attr.key != "exchange_rates" {
                            continue;
                        }
                        ex_rates = attr.value.clone();
                        break 'outer;
                    }
                }
            }
        }

        let mut rates = HashMap::new();
        if ex_rates.is_empty() {
            return Ok(rates);
        }
        let rate_splits: Vec<&str> = ex_rates.split(",").collect();
        for rate in rate_splits.iter() {
            let ss: Vec<&str> = rate.splitn(2, "u").collect();
            if ss.len() != 2 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("invalid exchange_rates {:?}", rate),
                ));
            }

            let mut k = String::from("u");
            k.push_str(ss[1].to_owned().as_str());

            let v: f64 = ss[0].parse().unwrap();

            rates.insert(k, v);
        }

        rates.insert(String::from("uluna"), 1.0);
        Ok(rates)
    }

    pub fn get_supply(&self) -> io::Result<HashMap<String, u64>> {
        let data = match &self.data {
            Some(v) => v,
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("no data found for {}", self.chain_id),
                ))
            }
        };

        let mut supplies = HashMap::new();
        for sp in data.supply.iter() {
            let k = sp.denom.to_string();
            let v: u64 = sp.amount.parse().unwrap();
            supplies.insert(k, v);
        }
        Ok(supplies)
    }
}
