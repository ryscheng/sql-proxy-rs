extern crate abci;
extern crate byteorder;
extern crate env_logger;
extern crate futures;
#[macro_use]
extern crate log;
extern crate tokio;

use abci::*;
use env_logger::Env;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use std::env;
use std::net::SocketAddr;

// MariaDB Application.  Its only state is a SQL string as bytes.
struct MariaDBApp {
    sql: String,
}

impl MariaDBApp {
    fn new() -> MariaDBApp {
        MariaDBApp {
            sql: String::from(""),
        }
    }
}

// Convert incoming tx data to Sql string
fn convert_tx(tx: &[u8]) -> String {
    let sql = String::from_utf8(tx.to_vec()).unwrap();
    return sql;
}

impl abci::Application for MariaDBApp {
    // Validate transactions.  Rule: SQL string must be valid SQL
    fn check_tx(&mut self, req: &RequestCheckTx) -> ResponseCheckTx {
        let sql = convert_tx(req.get_tx());
        let dialect = GenericDialect {};
        let ast = Parser::parse_sql(&dialect, sql.clone());
        let mut resp = ResponseCheckTx::new();

        // TODO: cover sql injection
        // Validation Logic
        match ast {
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
        // Get the Tx [u8]
        let sql = convert_tx(req.get_tx());
        // Update state
        self.sql = sql;
        // Return default code 0 == bueno
        ResponseDeliverTx::new()
    }

    fn commit(&mut self, _req: &RequestCommit) -> ResponseCommit {
        // Create the response
        let mut resp = ResponseCommit::new();
        // Set data so last state is included in the block
        let bytes = self.sql.as_bytes();
        resp.set_data(bytes.to_vec());
        resp
    }
}

fn main() {
    // Run on localhost using default Tendermint port
    env_logger::from_env(Env::default().default_filter_or("info")).init();
    let bind_addr = env::args().nth(1).unwrap_or("0.0.0.0:26658".to_string());
    abci::run(bind_addr.parse().unwrap(), MariaDBApp::new());
}
