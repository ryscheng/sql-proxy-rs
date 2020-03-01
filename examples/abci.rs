extern crate abci;
extern crate byteorder;
extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate tokio;

use std::env;
use abci::*;
use env_logger::Env;
use mysql_async::prelude::*;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::{Parser};
use tokio::runtime::Runtime; 

// ABCI Application. Its only state is a SQL string as bytes.
struct AbciApp {
  sql: String,
}

impl AbciApp {
  fn new() -> AbciApp {
    AbciApp { sql: String::from("") }
  }
}

// Convert incoming tx data to Sql string
fn convert_tx(tx: &[u8]) -> String {
  let sql = String::from_utf8(tx.to_vec()).unwrap();
  return sql;
}

async fn run_query(sql: &String) -> Result<(), mysql_async::error::Error> {
  let database_url = "mysql://root:devpassword@127.0.0.1:3306/mariadb";
  let pool = mysql_async::Pool::new(database_url);
  let conn = pool.get_conn().await?;

  // Run query
  conn.drop_query(sql).await?;

  // The destructor of a connection will return it to the pool,
  // but pool should be disconnected explicitly because it's
  // an asynchronous procedure.
  pool.disconnect().await?;

  // the async fn returns Result, so
  Ok(())
}

impl abci::Application for AbciApp {
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
      },
      Err(_e) => { 
        info!("Invalid SQL");
        // Return error
        resp.set_code(1);
        resp.set_log(String::from("Must be valid sql!"));
      },
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
    // Run Query
    Runtime::new().unwrap().block_on(run_query(&self.sql));

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
    abci::run(bind_addr.parse().unwrap(), AbciApp::new());
}
