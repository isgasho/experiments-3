#![warn(clippy::all)]

use std::time::Duration;

mod server;

fn main() {
    // fix flags... clap has some dependency issue with tokio
    let srv = server::Handler::new(
        8001,
        Duration::from_secs(10),
        String::from("wss://observer.terra.dev"),
    );

    let future_task = srv.start();
    let rt = tokio::runtime::Runtime::new().unwrap();

    let ret = rt.block_on(future_task);
    match ret {
        Ok(_) => println!("server done"),
        Err(e) => println!("server aborted {}", e),
    };
}
