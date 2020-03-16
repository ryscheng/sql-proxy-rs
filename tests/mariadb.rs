#[macro_use]
extern crate log;

use futures::channel::oneshot;
use mysql::*;
use mysql::prelude::*;
use tokio::task::JoinHandle;

use mariadb_proxy::{
    packet::{DatabaseType, Packet},
    packet_handler::PacketHandler,
};

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

#[derive(Debug, PartialEq, Eq, Clone)]
struct Payment {
    customer_id: i32,
    amount: i32,
    account_name: Option<String>,
}

async fn initialize() -> oneshot::Sender<()> {
    let mut server = mariadb_proxy::server::Server::new(
        "0.0.0.0:3306".to_string(),
        DatabaseType::MariaDB,
        "mariadb-server:3306".to_string(),
    )
    .await;

    // Spawn server on separate task
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        info!("Proxy listening on: 0.0.0.0:3306");
        server.run(PassthroughHandler {}, rx).await;
    });
    tx
}

#[tokio::test]
async fn can_proxy_requests() -> Result<()> {
    let kill_switch = initialize().await;

    let database_uri = "mysql://root:devpassword@localhost:3306/testdb";
    let pool = Pool::new(database_uri).unwrap();
    let mut conn = pool.get_conn().unwrap();
    
    conn.query_drop(
        r"create temporary table payment (customer_id int not null, amount int not null, account_name text)")?;

    // Get the initial block height
    let initial_block_height: Option<i32> = conn
                .query_first(r"SELECT MAX(block_height) AS max_height FROM tendermint_blocks;")
                .unwrap();
    
    let payments = vec![
        Payment { customer_id: 1, amount: 2, account_name: None },
        Payment { customer_id: 3, amount: 4, account_name: Some("foo".into()) },
        Payment { customer_id: 5, amount: 6, account_name: None },
        Payment { customer_id: 7, amount: 8, account_name: None },
        Payment { customer_id: 9, amount: 10, account_name: Some("bar".into()) },
    ];

    // Insert data 
    conn.exec_batch(
        r"insert into payment (customer_id, amount, account_name) VALUES (:customer_id, :amount, :account_name)",
        payments.iter().map(|p| params! {
            "customer_id" => p.customer_id,
            "amount" => p.amount,
            "account_name" => &p.account_name,
        })
    )?;

    // Grab data from db
    let selected_payments = conn
        .query_map(
            "SELECT customer_id, amount, account_name from payment",
            |(customer_id, amount, account_name)| {
                Payment { customer_id, amount, account_name }
            },
        )?;

    // Assert data is correct
    assert_eq!(payments, selected_payments);

    let final_block_height: Option<i32> = conn
        .query_first(r"SELECT MAX(block_height) AS max_height FROM tendermint_blocks;")
        .unwrap();
    
    assert_eq!(final_block_height.unwrap(), initial_block_height.unwrap() + 2);

    kill_switch.send(());
    Ok(())
}
