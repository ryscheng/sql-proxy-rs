extern crate mariadb_proxy;

#[macro_use]
extern crate log;

use abci::*;
use env_logger;
use futures_util::future::FutureExt;
use hyper::{
    body::Body,
    client::{Client, HttpConnector},
    Uri,
};
use mysql::{Pool};
// use mysql_async;
use mariadb_proxy::{
    packet::{Packet, PacketType},
    packet_handler::PacketHandler,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
}
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

    fn new(bytes: &[u8]) -> Transaction {
        let txn_str = String::from_utf8(bytes.to_vec()).unwrap();
        let tokens = txn_str.split(DELIMITER).collect();
        if tokens.len() < 2 {
            resp.set_code(1);
            resp.set_log(String::from("Missing node_id or SQL query in transaction"));
            return resp;
        }
        Transaction {
            node_id: tokens[0].to_string(),
            sql: tokens[1].to_string(),
        }
    }
}

struct AbciApp {
    node_id: String,
    sql_pool: Pool,
    txn_queue: Vec<Transaction>,
    app_hash: String,
    hasher: Hasher,
    block_height: i64
}

impl AbciApp {
    fn new(node_id: String, sql_pool: Pool) -> AbciApp {
        AbciApp {
            node_id: node_id,
            sql_pool: sql_pool,
            txn_queue: Vec::new(),
            app_hash: "".to_string(),
            hasher: DefaultHasher::new(),
            block_height: 0,
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
        for row in pool.prep_exec("SELECT MAX(block_height) AS max_height, app_hash  FROM `tendermint_blocks`;", ()).expect("SQL query failed to execute") {
            let (height, app_hash) = from_row(row.unwrap());
            self.block_height = height;
            self.hasher = DefaultHasher::new();
            self.app_hash = app_hash;
            self.app_hash.hash(&mut self.hasher); // Hash chaining
            response.set_last_block_height(self.block_height);
            response.set_last_block_app_hash(self.app_hash.into_bytes());
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
        let txn = Transaction::new(req.get_tx());

        // Parse SQL
        info!("Checking Transaction: Sql query: {}", txn.sql);
        let dialect = GenericDialect {};
        match Parser::parse_sql(&dialect, txn.sql.clone()) {
            Ok(_val) => {
                info!("Valid SQL");
                resp.set_code(0);
            }
            Err(_e) => {
                info!("Invalid SQL");
                resp.set_code(1);  // Return error
                resp.set_log(String::from("Must be valid sql!"));
            }
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
        self.hasher = DefaultHasher::new();
        self.app_hash.hash(&mut self.hasher); // Hash chaining
        self.block_height += 1;
        self.txn_queue.clear();
        self.txn_queue.push(Transaction::new("abci".to_string(), "START TRANSACTION;"));

        // PostgresSQL:
        //self.txn_queue.push(Transaction::new("abci".to_string(), "BEGIN;"));
        ResponseBeginBlock::new()
    }

    // Transaction = 1 SQL query
    // Process the SQL query
    fn deliver_tx(&mut self, req: &RequestDeliverTx) -> ResponseDeliverTx {
        info!("ABCI: deliver_tx()");
        let txn = Transaction::new(req.get_tx());
        txn.sql.hash(&mut self.hasher);
        self.txn_queue.push(txn);
        ResponseDeliverTx::new()
    }

    /// Consensus Connection: Called at the end of the block.  Often used to update the validator set.
    fn end_block(&mut self, req: &RequestEndBlock) -> ResponseEndBlock {
        debug!("ABCI: end_block()");
        self.block_height = req.get_height();
        self.app_hash = self.hasher.finish().to_string();

        self.txn_queue.push(Transaction::new(
                "abci".to_string(), 
                "CREATE TABLE IF NOT EXISTS `tendermint_blocks` (`block_height` in PRIMARY KEY, `app_hash` varchar(20))`;"));
        self.txn_queue.push(Transaction::new(
                "abci".to_string(),
                "INSERT INTO `tendermint_blocks` VALUES (" + self.block_height.to_string() + ",`" + self.app_hash + "`);"));
        self.txn_queue.push(Transaction::new("abci".to_string(), "COMMIT;"));
        ResponseEndBlock::new()
    }

    fn commit(&mut self, _req: &RequestCommit) -> ResponseCommit {
        debug!("ABCI: commit()");

        // Create the response
        let mut resp = ResponseCommit::new();
        resp.set_data(self.app_hash.into_bytes()); // Return the app_hash to Tendermint to include in next block

        // Generate the SQL transaction
        let mut sql = "";
        for txn in &self.txn_queue {
            sql.push_str(txn.sql);
        }

        // Update state
        info!("Forwarding SQL: {}", sql);

        // https://docs.rs/mysql/17.0.0/mysql/struct.QueryResult.html
        let result = pool.prep_exec(sql, ()).expect("SQL query failed to execute");
        if self.node_id == txn.node_id {
            // TODO: route responses back to client socket
        }
        info!("Query successfully executed");

        // Return default code 0 == bueno
        resp
    }
}

struct ProxyHandler {
    node_id: String,
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
            info!("SQL: {}", sql);
            let mut url: String = "http://localhost:26657/broadcast_tx_commit?tx=".to_owned();
            url.push_str(&self.node_id);
            url.push_str(DELIMITER);
            url.push_str(&sql);
            info!("Pushing to Tendermint: {}", url);
            let _fut = self.http_client.get(url.parse().unwrap()).then(|res| {
                async move {
                    let response = res.unwrap();
                    debug!("Response: {}", response.status());
                    debug!("Headers: {:#?}\n", response.headers());
                }
            });
        } else {
            debug!("{:?} packet", p.packet_type());
        }

        //TODO: dynamic route 'write' requests
        //p.clone()
        Packet { bytes: Vec::new() }; // Dropping packets for now
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
    let db_uri_str = args.next().unwrap_or_else(|| "mysql://root:devpassword@mariadb:3306/mariadb".to_string());
    let db_uri = db_uri_str.parse::<Uri>().unwrap();
    // determint address for the ABCI application
    let abci_addr = args.next().unwrap_or("0.0.0.0:26658".to_string());

    let mut server = mariadb_proxy::server::Server::new(bind_addr.clone(), db_uri.host().unwrap().to_string()).await;
    info!("Proxy listening on: {}", bind_addr);
    abci::run(abci_addr.parse().unwrap(), AbciApp::new(node_id, Pool::new(db_uri_str).unwrap()));
    //server.run(ProxyHandler { node_id: node_id, http_client: Client::new() }).await;
    
}
