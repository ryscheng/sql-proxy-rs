extern crate mariadb_proxy;

extern crate abci;
extern crate byteorder;
extern crate env_logger;
extern crate futures;
extern crate futures_util;
extern crate hyper;
#[macro_use]
extern crate log;
extern crate mysql;
extern crate sqlparser;
extern crate tokio;

use abci::*;
use env_logger::Env;
use futures_util::future::FutureExt;
use hyper::{
    body::Body,
    client::{Client, HttpConnector},
};
use mysql as my;
use std::env;
// use mysql_async;
use mariadb_proxy::{
    packet::{Packet, PacketType},
    packet_handler::PacketHandler,
};
use sqlparser::{dialect::GenericDialect, parser::Parser};

// Convert incoming tx data to Sql string
fn convert_tx(tx: &[u8]) -> String {
    let sql = String::from_utf8(tx.to_vec()).unwrap();
    return sql;
}

fn run_query_sync(sql: String) {
    let database_url = "mysql://root:devpassword@mariadb:3306/mariadb";
    let pool = my::Pool::new(database_url).unwrap();

    info!("run_query_sync(): {}", sql);

    if sql.len() > 0 {
        let result = pool.prep_exec(sql, ());

        match result {
            Ok(_) => {
                info!("Query successfully executed");
            }
            Err(_e) => {
                info!("Query error: {}", _e);
            }
        }
    }
}

struct AbciApp {
    sql: String,
}

impl AbciApp {
    fn new() -> AbciApp {
        AbciApp {
            sql: String::from(""),
        }
    }
}

impl abci::Application for AbciApp {
    /// Query Connection: Called on startup from Tendermint.  The application should normally
    /// return the last know state so Tendermint can determine if it needs to replay blocks
    /// to the application.
    fn info(&mut self, _req: &RequestInfo) -> ResponseInfo {
        info!("info()");
        ResponseInfo::new()
    }

    /// Query Connection: Set options on the application (rarely used)
    fn set_option(&mut self, _req: &RequestSetOption) -> ResponseSetOption {
        info!("set_option()");
        ResponseSetOption::new()
    }

    /// Query Connection: Query your application. This usually resolves through a merkle tree holding
    /// the state of the app.
    fn query(&mut self, _req: &RequestQuery) -> ResponseQuery {
        info!("query()");
        ResponseQuery::new()
    }

    /// Consensus Connection:  Called once on startup. Usually used to establish initial (genesis)
    /// state.
    fn init_chain(&mut self, _req: &RequestInitChain) -> ResponseInitChain {
        info!("init_chain()");
        ResponseInitChain::new()
    }

    /// Consensus Connection: Called at the start of processing a block of transactions
    /// The flow is:
    /// begin_block()
    ///   deliver_tx()  for each transaction in the block
    /// end_block()
    /// commit()
    fn begin_block(&mut self, _req: &RequestBeginBlock) -> ResponseBeginBlock {
        info!("begin_block()");
        ResponseBeginBlock::new()
    }

    /// Consensus Connection: Called at the end of the block.  Often used to update the validator set.
    fn end_block(&mut self, _req: &RequestEndBlock) -> ResponseEndBlock {
        info!("end_block()");
        ResponseEndBlock::new()
    }

    // Validate transactions.  Rule: SQL string must be valid SQL
    fn check_tx(&mut self, req: &RequestCheckTx) -> ResponseCheckTx {
        info!("check_tx()");

        let sql = convert_tx(req.get_tx());
        info!("Sql query: {}", sql);

        let dialect = GenericDialect {};
        let mut resp = ResponseCheckTx::new();
        // TODO: cover sql injection
        match Parser::parse_sql(&dialect, sql.clone()) {
            Ok(_val) => {
                info!("Valid SQL");
                // Update state to keep state correct for next check_tx call
                self.sql = sql;
            }
            Err(_e) => {
                info!("Invalid SQL");
                // Return error
                resp.set_code(1);
                resp.set_log(String::from("Must be valid sql!"));
            }
        }
        return resp;
    }

    fn deliver_tx(&mut self, req: &RequestDeliverTx) -> ResponseDeliverTx {
        info!("deliver_tx()");

        // Get the Tx [u8]
        let sql = convert_tx(req.get_tx());
        // Update state
        self.sql = sql;
        // Return default code 0 == bueno
        ResponseDeliverTx::new()
    }

    fn commit(&mut self, _req: &RequestCommit) -> ResponseCommit {
        info!("commit() {}", self.sql);

        // Run Query
        //Runtime::new().unwrap().block_on(run_query_async(&self.sql));
        run_query_sync(self.sql.clone());

        // Create the response
        let mut resp = ResponseCommit::new();

        // Set data so last state is included in the block
        let bytes = self.sql.as_bytes();
        resp.set_data(bytes.to_vec());

        self.sql = String::from("");
        resp
    }
}

struct ProxyHandler {
    http_client: Client<HttpConnector, Body>,
}

// Just forward the packet
impl PacketHandler for ProxyHandler {
    fn handle_request(&mut self, p: &Packet) -> Packet {
        // Print out the packet
        //debug!("[{}]", String::from_utf8_lossy(&p.bytes));

        match p.packet_type() {
            // Forward all SQL queries to Tendermint
            Ok(PacketType::ComQuery) => {
                let payload = &p.bytes[5..];
                let sql = String::from_utf8(payload.to_vec()).expect("Invalid UTF-8");
                info!("SQL: {}", sql);
                let mut url: String = "http://localhost:26657/broadcast_tx_commit?tx=".to_owned();
                url.push_str(&sql);
                info!("Pushing to Tendermint: {}", url);
                let _fut = self.http_client.get(url.parse().unwrap()).then(|res| {
                    async move {
                        let response = res.unwrap();
                        debug!("Response: {}", response.status());
                        debug!("Headers: {:#?}\n", response.headers());
                    }
                });
            }
            _ => debug!("{:?} packet", p.packet_type()),
        }

        Packet {
            bytes: p.bytes.clone(),
        }
    }

    fn handle_response(&mut self, p: &Packet) -> Packet {
        Packet {
            bytes: p.bytes.clone(),
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::from_env(Env::default().default_filter_or("info")).init();

    info!("Tendermint MariaDB proxy... ");

    // determine address for the proxy to bind to
    let bind_addr = env::args().nth(1).unwrap_or("0.0.0.0:3306".to_string());
    // determine address of the MariaDB instance we are proxying for
    let db_addr = env::args().nth(2).unwrap_or("mariadb:3306".to_string());
    // determint address for the ABCI application
    let abci_addr = env::args().nth(2).unwrap_or("0.0.0.0:26658".to_string());

    let mut server = mariadb_proxy::server::Server::new(bind_addr.clone(), db_addr.clone()).await;
    info!("Proxy listening on: {}", bind_addr);
    abci::run(abci_addr.parse().unwrap(), AbciApp::new());
    //server.run(ProxyHandler { http_client: Client::new() }).await;
}
