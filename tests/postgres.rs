#[macro_use]
extern crate log;

use futures::channel::oneshot;
use std::{error::Error, sync::Once};
use tokio;
use tokio_postgres::{NoTls, SimpleQueryMessage};

use sql_proxy::{
    packet::{DatabaseType, Packet},
    packet_handler::PacketHandler,
};

static INIT: Once = Once::new();

struct PassthroughHandler {}

#[async_trait::async_trait]
impl PacketHandler for PassthroughHandler {
    async fn handle_request(&mut self, p: &Packet) -> Packet {
        debug!(
            "c=>s: {:?} packet: {} bytes",
            p.get_packet_type(),
            p.get_size()
        );
        p.clone()
    }

    async fn handle_response(&mut self, p: &Packet) -> Packet {
        debug!(
            "c<=s: {:?} packet: {} bytes",
            p.get_packet_type(),
            p.get_size()
        );
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
    INIT.call_once(|| {
        env_logger::init();
    });

    debug!("Constructing server");
    let mut server = sql_proxy::server::Server::new(
        "0.0.0.0:5432".to_string(),
        DatabaseType::PostgresSQL,
        "postgres-server:5432".to_string(),
    )
    .await;

    // Spawn server on separate task
    debug!("Spawning async server task");
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        info!("Proxy listening on: 0.0.0.0:5432");
        server.run(PassthroughHandler {}, rx).await;
    });
    debug!("async server task running");
    tx
}

#[tokio::test]
async fn postgres_can_proxy_requests() -> Result<(), Box<dyn Error>> {
    let kill_switch = initialize().await;

    debug!("SQL client to connect to proxy");
    let (client, connection) = tokio_postgres::connect(
        "postgresql://root:testpassword@localhost:5432/testdb",
        NoTls,
    )
    .await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });
    debug!("Initialized SQL client");

    client
        .batch_execute(
            "
        CREATE TEMPORARY TABLE person (
            id      SERIAL PRIMARY KEY,
            name    TEXT NOT NULL,
            gender  TEXT NOT NULL
        );
        ",
        )
        .await?;
    debug!("Created temporary table");

    client
        .batch_execute(
            "
        INSERT INTO person (name, gender) VALUES ('Alice', 'Female');
        INSERT INTO person (name, gender) VALUES ('Bob', 'Male');
        ",
        )
        .await?;
    debug!("Insert into payments");

    let rows = client
        .simple_query("SELECT name, gender FROM person;")
        .await?;
    debug!("Select from payments");

    if let SimpleQueryMessage::Row(row) = &rows[0] {
        assert_eq!(row.get(0).unwrap(), "Alice");
        assert_eq!(row.get(1).unwrap(), "Female");
    } else {
        panic!("Missing row[0]");
    }
    if let SimpleQueryMessage::Row(row) = &rows[1] {
        assert_eq!(row.get(0).unwrap(), "Bob");
        assert_eq!(row.get(1).unwrap(), "Male");
    } else {
        panic!("Missing row[1]");
    }

    debug!("Killing server");
    kill_switch.send(()).unwrap();
    Ok(())
}
