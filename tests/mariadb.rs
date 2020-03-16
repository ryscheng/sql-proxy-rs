#[macro_use]
extern crate log;

use env_logger;
use futures::channel::oneshot;
use mysql_async::prelude::*;
use std::{
    sync::Once,
    error::Error,
};

use mariadb_proxy::{
    packet::{DatabaseType, Packet},
    packet_handler::PacketHandler,
};

static INIT: Once = Once::new();

struct PassthroughHandler {}

#[async_trait::async_trait]
impl PacketHandler for PassthroughHandler {
    async fn handle_request(&mut self, p: &Packet) -> Packet {
        debug!("c=>s: {:?} packet: {} bytes", p.get_packet_type(), p.get_size());
        p.clone()
    }

    async fn handle_response(&mut self, p: &Packet) -> Packet {
        debug!("c<=s: {:?} packet: {} bytes", p.get_packet_type(), p.get_size());
        p.clone()
    }
}

async fn initialize() -> oneshot::Sender<()> {
    INIT.call_once(|| {
        env_logger::init();
    });

    debug!("Constructing server");
    let mut server = mariadb_proxy::server::Server::new(
        "0.0.0.0:3306".to_string(),
        DatabaseType::MariaDB,
        "mariadb-server:3306".to_string(),
    )
    .await;

    // Spawn server on separate task
    debug!("Spawning async server task");
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        info!("Proxy listening on: 0.0.0.0:3306");
        server.run(PassthroughHandler {}, rx).await;
    });
    debug!("async server task running");
    tx
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct Payment {
    customer_id: i32,
    amount: i32,
    account_name: Option<String>,
}

#[tokio::test]
async fn can_proxy_requests() -> Result<(), Box<dyn Error>> {
    let kill_switch = initialize().await;

    debug!("SQL client to connect to proxy");
    let database_uri = "mysql://root:devpassword@localhost:3306/testdb";
    let pool = mysql_async::Pool::new(database_uri);
    debug!("SQL client get connection");
    let conn = pool.get_conn().await?;
    debug!("Initialized SQL client");
    
    let conn = conn.drop_query(
        r"create temporary table payment (customer_id int not null, amount int not null, account_name text)").await?;
    debug!("Created temporary table");

    // Get the initial block height
    //let initial_block_height: Option<i32> = conn
    //            .query_first(r"SELECT MAX(block_height) AS max_height FROM tendermint_blocks;").await?;
    //debug!("Get initial block height");
    
    let payments = vec![
        Payment { customer_id: 1, amount: 2, account_name: None },
        Payment { customer_id: 3, amount: 4, account_name: Some("foo".into()) },
        Payment { customer_id: 5, amount: 6, account_name: None },
        Payment { customer_id: 7, amount: 8, account_name: None },
        Payment { customer_id: 9, amount: 10, account_name: Some("bar".into()) },
    ];
    let payments_clone = payments.clone();

    // Insert data 
    let conn = conn.batch_exec(
        r"insert into payment (customer_id, amount, account_name) VALUES (:customer_id, :amount, :account_name)",
        payments_clone.into_iter().map(|p| params! {
            "customer_id" => p.customer_id,
            "amount" => p.amount,
            "account_name" => p.account_name.clone(),
        })
    ).await?;
    debug!("Insert into payments");

    // Grab data from db
    let result = conn.prep_exec("SELECT customer_id, amount, account_name from payment", ()).await?;
    let (_conn, selected_payments) = result.map_and_drop(|row| {
      let (customer_id, amount, account_name) = mysql_async::from_row(row);
      Payment { customer_id, amount, account_name }
    }).await?;
    debug!("Select from payments");

    // Assert data is correct
    assert_eq!(payments, selected_payments);

    //let final_block_height: Option<i32> = conn
    //    .query_first(r"SELECT MAX(block_height) AS max_height FROM tendermint_blocks;")
    //    .unwrap();
    //debug!("Get tendermint height");
    
    //assert_eq!(final_block_height.unwrap(), initial_block_height.unwrap() + 2);

    debug!("Killing server");
    kill_switch.send(()).unwrap();
    Ok(())
}
