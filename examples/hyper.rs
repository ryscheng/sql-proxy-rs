#[macro_use]
extern crate log;

use futures::executor::block_on;
use hyper::client::Client;

fn run() {
    let url: String = String::from("http://httpbin.org/ip");

    block_on(async {
        let res = Client::new().get(url.parse().unwrap()).await;
        info!("inside async");
        match res {
            Ok(response) => {
                info!("Response: {}", response.status());
                info!("Headers: {:#?}\n", response.headers());
            }
            Err(e) => {
                warn!("Unable to forward to Tendermint: {}", e);
            }
        }
    });
}

#[tokio::main]
async fn main() {
    env_logger::init();

    info!("Hyper demo");
    run();
}
