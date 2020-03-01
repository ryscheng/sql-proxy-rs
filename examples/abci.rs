extern crate abci;
extern crate byteorder;
extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate tokio;

use std::env;
use abci::*;
use env_logger::Env;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::{Parser};
// use tokio::runtime::Runtime;

use mysql;
// use mysql_async;
// use mysql::prelude::*;

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

// async fn run_query_async(sql: &String) -> Result<(), mysql_async::error::Error> {
//   let database_url = "mysql://root:devpassword@127.0.0.1:3306/mariadb";
//   let pool = mysql_async::Pool::new(database_url);
//   let conn = pool.get_conn().await?;

//   info!("run_query_async(): {}", sql);

//   // Run query
//   conn.drop_query(sql).await?;

//   // The destructor of a connection will return it to the pool,
//   // but pool should be disconnected explicitly because it's
//   // an asynchronous procedure.
//   pool.disconnect().await?;

//   // the async fn returns Result, so
//   Ok(())
// }

fn run_query_sync(sql: String) {
  let database_url = "mysql://root:devpassword@mariadb:3306/mariadb";
  let pool = mysql::Pool::new(database_url).unwrap();

  info!("run_query_sync(): {}", sql);

  if sql.len() > 0 {
    let result = pool.prep_exec(sql, ());

    match result {
      Ok(_) => {
        info!("Query successfully executed");
      },
      Err(_e) => {
        info!("Query error: {}", _e);
      },
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
    let dialect = GenericDialect {};
    let ast = Parser::parse_sql(&dialect, sql.clone());
    let mut resp = ResponseCheckTx::new();

    info!("Sql query: {}", sql);

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
    // Runtime::new().unwrap().block_on(run_query_async(&self.sql));
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

fn main() {
  // Run on localhost using default Tendermint port
  env_logger::from_env(Env::default().default_filter_or("info")).init();
  let bind_addr = env::args().nth(1).unwrap_or("0.0.0.0:26658".to_string());
  abci::run(bind_addr.parse().unwrap(), AbciApp::new());
}
