#[macro_use]
extern crate log;

use env_logger;
use postgres::{Client, NoTls};
use mariadb_proxy::{
    packet::{DatabaseType, Packet},
    packet_handler::PacketHandler,
};
use tokio::task::JoinHandle;

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

async fn initialize() -> JoinHandle<()> {
    let mut server = mariadb_proxy::server::Server::new(
        "0.0.0.0:5432".to_string(),
        DatabaseType::PostgresSQL,
        "postgres-server:5432".to_string(),
    )
    .await;

    // Spawn server on separate task
    //let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        info!("Proxy listening on: 0.0.0.0:5432");
        server.run(PassthroughHandler {}).await;
    })
}

#[tokio::test]
async fn can_proxy_requests() {
    let kill = initialize().await;

    let mut client = Client::connect("postgresql://postgres:devpassword@mariadb-proxy:5432/testdb", NoTls).unwrap();

    client.batch_execute("
        CREATE TEMPORARY TABLE person (
            id      SERIAL PRIMARY KEY,
            name    TEXT NOT NULL,
            gender  TEXT NOT NULL
        )
    ").unwrap();

    client.execute(
        "INSERT INTO person (name, gender) VALUES ($1, $2)",
        &[&"Alice", &"Female"]
    ).unwrap();

    client.execute(
        "INSERT INTO person (name, gender) VALUES ($1, $2)",
        &[&"Bob", &"Male"]
    ).unwrap();

    let rows = client.query("SELECT name, gender FROM person", &[]).unwrap();

    // Assert data is correct
    assert_eq!(rows[0].get::<_, &str>(0), "Alice");
    assert_eq!(rows[0].get::<_, &str>(1), "Female");
    assert_eq!(rows[1].get::<_, &str>(0), "Bob");
    assert_eq!(rows[1].get::<_, &str>(1), "Male");
}
