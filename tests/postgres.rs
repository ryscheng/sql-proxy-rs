#[macro_use]
extern crate log;

use env_logger;
use postgres::{Client, NoTls};

#[derive(Debug, PartialEq, Eq, Clone)]
struct Payment {
    customer_id: i32,
    amount: i32,
    account_name: Option<String>,
}

fn can_proxy_requests_to_tendermint() {
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
