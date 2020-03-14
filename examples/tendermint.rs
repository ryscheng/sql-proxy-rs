extern crate mariadb_proxy;

#[macro_use]
extern crate log;

use abci::*;
use env_logger;
use hex;
use http::uri::Uri;
use hyper;
use mariadb_proxy::{
    packet::{DatabaseType, Packet},
    packet_handler::PacketHandler,
};
use mysql::{from_row, from_value, Pool, Value};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sodiumoxide::crypto::hash;
use sqlparser::{dialect::GenericDialect, parser::Parser};
use std::io::{Error, ErrorKind};
use tokio;
use urlencoding;

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

    fn decode(s: String) -> Result<Transaction, Error> {
        if let Ok(contents) = urlencoding::decode(&s) {
            let tokens: Vec<&str> = contents.split(DELIMITER).collect();
            if tokens.len() < 2 {
                Err(Error::new(
                    ErrorKind::Other,
                    "Missing node_id or SQL query in transaction",
                ))
            } else {
                Ok(Transaction {
                    node_id: tokens[0].to_string(),
                    sql: tokens[1].to_string(),
                })
            }
        } else {
            return Err(Error::new(ErrorKind::Other, "Unable to decode transaction"));
        }
    }

    fn encode(&self) -> String {
        let mut contents = String::from("");
        contents.push_str(&self.node_id);
        contents.push_str(DELIMITER);
        contents.push_str(&self.sql);
        urlencoding::encode(&contents)
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
            app_hash: "start".to_string(),
        }
    }
}

impl Application for AbciApp {
    /// Query Connection: Called on startup from Tendermint.  The application should normally
    /// return the last know state so Tendermint can determine if it needs to replay blocks
    /// to the application.
    fn info(&mut self, _req: &RequestInfo) -> ResponseInfo {
        debug!("ABCI:info()");
        let mut response = ResponseInfo::new();
        let sql_query = "SELECT MAX(block_height) AS max_height, app_hash FROM tendermint_blocks;";
        match self.sql_pool.prep_exec(sql_query, ()) {
            Ok(rows) => {
                for row in rows {
                    let (height, app_hash) = from_row(row.unwrap());

                    // Check for null values when tendermint_blocks table is empty
                    self.block_height = match height {
                        Value::NULL => 0,
                        _ => from_value::<i64>(height),
                    };
                    self.app_hash = match app_hash {
                        Value::NULL => "".to_string(),
                        _ => from_value::<String>(app_hash),
                    };

                    response.set_last_block_height(self.block_height);
                    response.set_last_block_app_hash(self.app_hash.clone().into_bytes());
                }
            }
            Err(e) => warn!("SQL query failed to execute: {}", e),
        }

        response
    }

    /// Query Connection: Set options on the application (rarely used)
    fn set_option(&mut self, _req: &RequestSetOption) -> ResponseSetOption {
        debug!("ABCI:set_option()");
        ResponseSetOption::new()
    }

    /// Query Connection: Query your application. This usually resolves through a merkle tree holding
    /// the state of the app.
    fn query(&mut self, _req: &RequestQuery) -> ResponseQuery {
        debug!("ABCI:query()");
        ResponseQuery::new()
    }

    /// Consensus Connection:  Called once on startup. Usually used to establish initial (genesis)
    /// state.
    fn init_chain(&mut self, _req: &RequestInitChain) -> ResponseInitChain {
        debug!("ABCI:init_chain()");
        ResponseInitChain::new()
    }

    // Validate transactions.  Rule: SQL string must be valid SQL
    fn check_tx(&mut self, req: &RequestCheckTx) -> ResponseCheckTx {
        debug!("ABCI:check_tx()");
        let mut resp = ResponseCheckTx::new();

        if let Ok(enc_txn) = String::from_utf8(req.get_tx().to_vec()) {
            if let Ok(txn) = Transaction::decode(enc_txn) {
                info!(
                    "ABCI:check_tx(): Checking Transaction: Sql query: {}",
                    txn.sql
                );
                // Parse SQL
                let dialect = GenericDialect {};
                if let Ok(_val) = Parser::parse_sql(&dialect, txn.sql.clone()) {
                    info!("ABCI:check_tx(): Valid SQL");
                    resp.set_code(0);
                } else {
                    warn!("ABCI:check_tx(): Invalid SQL");
                    resp.set_code(1); // Return error
                    resp.set_log(String::from("Must be valid sql!"));
                }
            } else {
                warn!("ABCI:check_tx(): Unable to decode transaction");
                resp.set_code(1); // Return error
                resp.set_log(String::from("Must be valid transaction!"));
            }
        } else {
            warn!("ABCI:check_tx(): Invalid transaction");
            resp.set_code(1); // Return error
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
        debug!("ABCI:begin_block()");
        self.block_height += 1;
        self.txn_queue.clear();
        ResponseBeginBlock::new()
    }

    // Transaction = 1 SQL query
    // Process the SQL query
    fn deliver_tx(&mut self, req: &RequestDeliverTx) -> ResponseDeliverTx {
        debug!("ABCI:deliver_tx()");

        if let Ok(enc_txn) = String::from_utf8(req.get_tx().to_vec()) {
            if let Ok(txn) = Transaction::decode(enc_txn) {
                let digest = hash::hash((self.app_hash.clone() + &txn.sql).as_bytes()); // Hash chaining
                self.app_hash = hex::encode(digest.as_ref()); // Store as hexcode
                self.txn_queue.push(txn);
                info!("ABCI:deliver_tx(): Pushing txn. app_hash={}", self.app_hash);
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
        debug!("ABCI:end_block()");

        self.block_height = req.get_height();

        // Add mandatory transactions to queue
        self.txn_queue.push(Transaction::new(
                "abci".to_string(), 
                "CREATE TABLE IF NOT EXISTS tendermint_blocks (block_height int PRIMARY KEY, app_hash text);".to_string()));
        self.txn_queue.push(Transaction::new(
            "abci".to_string(),
            "INSERT INTO tendermint_blocks VALUES (".to_string()
                + &self.block_height.to_string()
                + ",\""
                + &self.app_hash
                + "\");",
        ));

        ResponseEndBlock::new()
    }

    fn commit(&mut self, _req: &RequestCommit) -> ResponseCommit {
        debug!("ABCI:commit()");

        // Create the response
        let mut resp = ResponseCommit::new();
        resp.set_data(self.app_hash.clone().into_bytes()); // Return the app_hash to Tendermint to include in next block

        // Generate SQL transaction
        let mut tx = self.sql_pool.start_transaction(false, None, None).unwrap();

        for txn in &self.txn_queue {
            info!("ABCI:commit(): Forwarding SQL: {}", &txn.sql);
            tx.prep_exec(&txn.sql, ()).unwrap();
        }

        // Update state
        tx.commit().unwrap();

        // TODO: route responses back to client socket
        //if self.node_id == txn.node_id {
        //}
        info!("ABCI:commit(): Query successfully executed");

        // Return default code 0 == bueno
        resp
    }
}

struct ProxyHandler {
    node_id: String,
    db_type: DatabaseType,
    tendermint_addr: String,
    http_client: hyper::Client<hyper::client::HttpConnector, hyper::Body>,
}

impl ProxyHandler {
    fn new(node_id: String, db_type: DatabaseType, tendermint_addr: String) -> ProxyHandler {
        ProxyHandler {
            node_id: node_id,
            db_type: db_type,
            tendermint_addr: tendermint_addr,
            http_client: hyper::Client::new(),
        }
    }
}

// Just forward the packet
#[async_trait::async_trait]
impl PacketHandler for ProxyHandler {
    async fn handle_request(&mut self, p: &Packet) -> Packet {
        // Print out the packet
        //debug!("[{}]", String::from_utf8_lossy(&p.bytes));

        if let Ok(sql) = p.get_query() {
            let txn = Transaction::new(self.node_id.clone(), sql.clone());
            info!("SQL: {}", sql);

            //dynamic route only write requests
            let lower_sql = sql.to_lowercase();
            if lower_sql.contains("create")
                || lower_sql.contains("insert")
                || lower_sql.contains("update")
                || lower_sql.contains("delete")
            {
                let mut uri_str: String = String::from("http://");
                uri_str.push_str(&self.tendermint_addr);
                uri_str.push_str("/broadcast_tx_commit?tx=");
                uri_str.push_str("%22");
                uri_str.push_str(txn.encode().as_str());
                uri_str.push_str("%22"); //uri_str.push_str("\"");
                info!("Pushing to Tendermint: {}", uri_str);
                //let uri_str = "http://httpbin.org/ip";
                let uri = uri_str
                    .parse()
                    .expect(format!("Unable to parse URL {}", uri_str).as_str());
                let response = self
                    .http_client
                    .get(uri)
                    .await
                    .expect("HTTP GET request failed");
                info!("Response: {}", response.status());
                info!("Headers: {:#?}\n", response.headers());
                let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
                info!(
                    "Body: {:#?}\n",
                    String::from_utf8(body_bytes.to_vec()).expect("response was not valid utf-8")
                );
                return Packet::new(self.db_type, Vec::new()); // Dropping packets for now
            }
        }

        // Default case: forward packet
        debug!("{:?} packet", p.get_packet_type());
        p.clone()
    }

    async fn handle_response(&mut self, p: &Packet) -> Packet {
        p.clone()
    }
}

#[tokio::main]
async fn main() {
    env_logger::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let node_id: String = thread_rng().sample_iter(&Alphanumeric).take(16).collect();

    info!("Tendermint MariaDB proxy (node_id={}) ... ", node_id);

    let mut args = std::env::args().skip(1);
    // determine address for the proxy to bind to
    let bind_addr = args.next().unwrap_or_else(|| "0.0.0.0:3306".to_string());
    // determine address of the database we are proxying for
    let db_uri_str = args
        .next()
        .unwrap_or_else(|| "mysql://root:devpassword@mariadb-server:3306/testdb".to_string());
    let db_uri = db_uri_str.parse::<Uri>().unwrap();
    let db_addr =
        db_uri.host().unwrap().to_string() + ":" + &db_uri.port_u16().unwrap().to_string();
    // determint address for the ABCI application
    let abci_addr = args.next().unwrap_or("0.0.0.0:26658".to_string());
    let tendermint_addr = args.next().unwrap_or("tendermint-node:26657".to_string());

    // Start proxy server
    // let handler = ProxyHandler { node_id: node_id.clone(), tendermint_addr: tendermint_addr, http_client: Client::new() };
    let handler = ProxyHandler::new(node_id.clone(), DatabaseType::PostgresSQL, tendermint_addr);
    let mut server = mariadb_proxy::server::Server::new(
        bind_addr.clone(),
        DatabaseType::PostgresSQL,
        db_addr.clone(),
    )
    .await;

    tokio::spawn(async move {
        info!("Proxy listening on: {}", bind_addr);
        server.run(handler).await;
    });

    // Start ABCI application
    info!("ABCI application listening on: {}", abci_addr);
    abci::run(
        abci_addr.parse().unwrap(),
        AbciApp::new(node_id.clone(), Pool::new(db_uri_str).unwrap()),
    );
}
