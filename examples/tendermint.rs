extern crate mariadb_proxy;

#[macro_use]
extern crate log;

use abci::*;
use base64;
use env_logger;
use futures_util::future::FutureExt;
use hyper::{
    body::Body,
    client::{Client, HttpConnector},
    Uri,
};
use mysql::{Pool, from_row};
// use mysql_async;
use mariadb_proxy::{
    packet::{Packet, PacketType},
    packet_handler::{PacketHandler},
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sodiumoxide::crypto::hash;
use std::{
    error::{Error},
    io,
};
use sqlparser::{dialect::GenericDialect, parser::Parser};
use tokio;

const DELIMITER: &str = "!_!";

struct Transaction {
    pub node_id: String,
    pub sql: String,
}

impl Transaction {
    fn new(node_id: String, sql: String) -> Transaction {
        Transaction {
            node_id: node_id,
            sql: sql,
        }
    }

    fn decode(s: String) -> Result<Transaction, Box<dyn Error>>{
        let bytes = base64::decode(&s)?;
        let contents = String::from_utf8(bytes)?;
        let tokens: Vec<&str> = contents.split(DELIMITER).collect();
        if tokens.len() < 2 {
            Err(Box::new(io::Error::new(io::ErrorKind::Other, "Missing node_id or SQL query in transaction")))
        } else {
            Ok(Transaction {
                node_id: tokens[0].to_string(),
                sql: tokens[1].to_string(),
            })
        }

    }

    fn encode(&self) -> String {
        let mut contents = String::from("");
        contents.push_str(&self.node_id);
        contents.push_str(DELIMITER);
        contents.push_str(&self.sql);
        base64::encode(&contents)
    }

}

struct AbciApp {
    node_id: String,
    sql_pool: Pool,
    txn_queue: Vec<Transaction>,
    block_height: i64,
    app_hash: String,
}

impl AbciApp {
    fn new(node_id: String, sql_pool: Pool) -> AbciApp {
        AbciApp {
            node_id: node_id,
            sql_pool: sql_pool,
            txn_queue: Vec::new(),
            block_height: 0,
            app_hash: "".to_string(),
        }
    }
}

impl Application for AbciApp {
    /// Query Connection: Called on startup from Tendermint.  The application should normally
    /// return the last know state so Tendermint can determine if it needs to replay blocks
    /// to the application.
    fn info(&mut self, _req: &RequestInfo) -> ResponseInfo {
        debug!("ABCI: info()");
        let mut response = ResponseInfo::new();
        let sql_query = "SELECT MAX(block_height) AS max_height, app_hash  FROM `tendermint_blocks`;";
        match self.sql_pool.prep_exec(sql_query, ()) {
            Ok(rows) => {
                for row in rows {
                    let (height, app_hash) = from_row(row.unwrap());
                    self.block_height = height;
                    self.app_hash = app_hash;
                    response.set_last_block_height(self.block_height);
                    response.set_last_block_app_hash(self.app_hash.clone().into_bytes());
                }
            },
            Err(e) => warn!("SQL query failed to execute: {}", e),
        }

        response
    }

    /// Query Connection: Set options on the application (rarely used)
    fn set_option(&mut self, _req: &RequestSetOption) -> ResponseSetOption {
        debug!("ABCI: set_option()");
        ResponseSetOption::new()
    }

    /// Query Connection: Query your application. This usually resolves through a merkle tree holding
    /// the state of the app.
    fn query(&mut self, _req: &RequestQuery) -> ResponseQuery {
        debug!("ABCI: query()");
        ResponseQuery::new()
    }

    /// Consensus Connection:  Called once on startup. Usually used to establish initial (genesis)
    /// state.
    fn init_chain(&mut self, _req: &RequestInitChain) -> ResponseInitChain {
        debug!("ABCI: init_chain()");
        ResponseInitChain::new()
    }

    // Validate transactions.  Rule: SQL string must be valid SQL
    fn check_tx(&mut self, req: &RequestCheckTx) -> ResponseCheckTx {
        debug!("ABCI: check_tx()");
        let mut resp = ResponseCheckTx::new();

        if let Ok(enc_txn) = String::from_utf8(req.get_tx().to_vec()) {
            if let Ok(txn) = Transaction::decode(enc_txn) {
                info!("Checking Transaction: Sql query: {}", txn.sql);
                // Parse SQL
                let dialect = GenericDialect {};
                if let Ok(_val) = Parser::parse_sql(&dialect, txn.sql.clone()) {
                    info!("Valid SQL");
                    resp.set_code(0);
                } else {
                    warn!("Invalid SQL");
                    resp.set_code(1);  // Return error
                    resp.set_log(String::from("Must be valid sql!"));
                }
            } else {
                warn!("Unable to decode transaction");
                resp.set_code(1);  // Return error
                resp.set_log(String::from("Must be valid transaction!"));
            }
        } else {
            warn!("Invalid transaction");
            resp.set_code(1);  // Return error
            resp.set_log(String::from("Must be valid transaction!"));
        }

        

        return resp;
    }

    /// Consensus Connection: Called at the start of processing a block of transactions
    /// The flow is:
    /// begin_block()
    ///   deliver_tx()  for each transaction in the block
    /// end_block()
    /// commit()
    fn begin_block(&mut self, _req: &RequestBeginBlock) -> ResponseBeginBlock {
        debug!("ABCI: begin_block()");
        self.block_height += 1;
        self.txn_queue.clear();
        self.txn_queue.push(Transaction::new("abci".to_string(), "START TRANSACTION;".to_string()));

        // PostgresSQL:
        //self.txn_queue.push(Transaction::new("abci".to_string(), "BEGIN;"));
        ResponseBeginBlock::new()
    }

    // Transaction = 1 SQL query
    // Process the SQL query
    fn deliver_tx(&mut self, req: &RequestDeliverTx) -> ResponseDeliverTx {
        info!("ABCI: deliver_tx()");

        if let Ok(enc_txn) = String::from_utf8(req.get_tx().to_vec()) {
            if let Ok(txn) = Transaction::decode(enc_txn) {
                let digest = hash::hash((self.app_hash.clone() + &txn.sql).as_bytes());     // Hash chaining
                self.app_hash = String::from(format!("{:x?}", digest.as_ref()));    // Store as hexcode
                self.txn_queue.push(txn);
            } else {
                warn!("unable to decode transaction at deliver_tx()");
            }
        } else {
            warn!("invalid transaction at deliver_tx()");
        }

        ResponseDeliverTx::new()
    }

    /// Consensus Connection: Called at the end of the block.  Often used to update the validator set.
    fn end_block(&mut self, req: &RequestEndBlock) -> ResponseEndBlock {
        debug!("ABCI: end_block()");
        self.block_height = req.get_height();

        self.txn_queue.push(Transaction::new(
                "abci".to_string(), 
                "CREATE TABLE IF NOT EXISTS `tendermint_blocks` (`block_height` in PRIMARY KEY, `app_hash` varchar(20))`;".to_string()));
        self.txn_queue.push(Transaction::new(
                "abci".to_string(),
                "INSERT INTO `tendermint_blocks` VALUES (".to_string() + &self.block_height.to_string() + ",`" + &self.app_hash + "`);"));
        self.txn_queue.push(Transaction::new("abci".to_string(), "COMMIT;".to_string()));
        ResponseEndBlock::new()
    }

    fn commit(&mut self, _req: &RequestCommit) -> ResponseCommit {
        debug!("ABCI: commit()");

        // Create the response
        let mut resp = ResponseCommit::new();
        resp.set_data(self.app_hash.clone().into_bytes()); // Return the app_hash to Tendermint to include in next block

        // Generate the SQL transaction
        let mut sql = "".to_string();
        for txn in &self.txn_queue {
            sql.push_str(&txn.sql);
        }

        // Update state
        info!("Forwarding SQL: {}", sql);

        // https://docs.rs/mysql/17.0.0/mysql/struct.QueryResult.html
        // TODO: commit to database
        //let _result = self.sql_pool.prep_exec(sql, ()).expect("SQL query failed to execute");
        // TODO: route responses back to client socket
        //if self.node_id == txn.node_id {
        //}
        info!("Query successfully executed");

        // Return default code 0 == bueno
        resp
    }
}

struct ProxyHandler {
    node_id: String,
    tendermint_addr: String,
    http_client: Client<HttpConnector, Body>,
}

// Just forward the packet
impl PacketHandler for ProxyHandler {
    fn handle_request(&mut self, p: &Packet) -> Packet {
        // Print out the packet
        //debug!("[{}]", String::from_utf8_lossy(&p.bytes));

        if let Ok(PacketType::ComQuery) = p.packet_type() {
            let payload = &p.bytes[5..];
            let sql = String::from_utf8(payload.to_vec()).expect("Invalid UTF-8");
            let txn = Transaction::new(self.node_id.clone(), sql.clone());
            info!("SQL: {}", sql);

            //dynamic route only write requests
            let lower_sql = sql.to_lowercase();
            if lower_sql.contains("create") || 
                    lower_sql.contains("insert") || 
                    lower_sql.contains("update") || 
                    lower_sql.contains("delete") {
                let mut url: String = String::from("http://");
                url.push_str(&self.tendermint_addr);
                url.push_str("/broadcast_tx_commit?tx=");
                url.push_str(txn.encode().as_str());
                info!("Pushing to Tendermint: {}", url);
                let _fut = self.http_client.get(url.parse().unwrap()).then(|res| {
                    async move {
                        match res {
                            Ok(response) => {
                                debug!("Response: {}", response.status());
                                debug!("Headers: {:#?}\n", response.headers());
                            },
                            Err(e) => {
                                warn!("Unable to forward to Tendermint: {}", e);
                            },
                        }
                    }
                });
                return Packet { bytes: Vec::new() }; // Dropping packets for now
            }
        }

        // Default case: forward packet
        debug!("{:?} packet", p.packet_type());
        p.clone()
    }

    fn handle_response(&mut self, p: &Packet) -> Packet {
        p.clone()
    }
}

#[tokio::main]
async fn main() {
    env_logger::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let node_id: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .collect();

    info!("Tendermint MariaDB proxy (node_id={}) ... ", node_id);

    let mut args = std::env::args().skip(1);
    // determine address for the proxy to bind to
    let bind_addr = args.next().unwrap_or_else(|| "0.0.0.0:3306".to_string());
    // determine address of the database we are proxying for
    let db_uri_str = args.next().unwrap_or_else(|| "mysql://root:devpassword@mariadb:3306/testdb".to_string());
    let db_uri = db_uri_str.parse::<Uri>().unwrap();
    let db_addr = db_uri.host().unwrap().to_string() + ":" + &db_uri.port().unwrap().to_string();
    // determint address for the ABCI application
    let abci_addr = args.next().unwrap_or("0.0.0.0:26658".to_string());
    let tendermint_addr = args.next().unwrap_or("tendermint:26657".to_string());

    // Start proxy server
    let handler = ProxyHandler { node_id: node_id.clone(), tendermint_addr: tendermint_addr, http_client: Client::new() };
    let mut server = mariadb_proxy::server::Server::new(bind_addr.clone(), db_addr.clone()).await;
    tokio::spawn(async move {
        info!("Proxy listening on: {}", bind_addr);
        server.run(handler).await;
    });
    
    // Start ABCI application
    info!("ABCI application listening on: {}", abci_addr);
    abci::run(abci_addr.parse().unwrap(), AbciApp::new(node_id.clone(), Pool::new(db_uri_str).unwrap()));
}
