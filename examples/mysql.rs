extern crate abci;
extern crate byteorder;
extern crate env_logger;
#[macro_use] extern crate log;
extern crate mysql_async;
extern crate tokio;

use mysql_async::prelude::*;

#[derive(Debug, PartialEq, Eq, Clone)]
struct Payment {
    customer_id: i32,
    amount: i32,
    account_name: Option<String>,
}

async fn insert() -> Result<(), mysql_async::error::Error> {
    let payments = vec![
        Payment { customer_id: 1, amount: 2, account_name: None },
        Payment { customer_id: 3, amount: 4, account_name: Some("foo".into()) },
        Payment { customer_id: 5, amount: 6, account_name: None },
        Payment { customer_id: 7, amount: 8, account_name: None },
        Payment { customer_id: 9, amount: 10, account_name: Some("bar".into()) },
    ];
    let payments_clone = payments.clone();

    let database_url = "mysql://root:devpassword@127.0.0.1:3306/mariadb";
    let pool = mysql_async::Pool::new(database_url);
    let conn = pool.get_conn().await?;

    // Create temporary table
    let conn = conn.drop_query(
        r"CREATE TEMPORARY TABLE payment (
            customer_id int not null,
            amount int not null,
            account_name text
        )"
    ).await?;

    // Save payments
    let params = payments_clone.into_iter().map(|payment| {
        params! {
            "customer_id" => payment.customer_id,
            "amount" => payment.amount,
            "account_name" => payment.account_name.clone(),
        }
    });

    let conn = conn.batch_exec(r"INSERT INTO payment (customer_id, amount, account_name)
                    VALUES (:customer_id, :amount, :account_name)", params).await?;

    // Load payments from database.
    let result = conn.prep_exec("SELECT customer_id, amount, account_name FROM payment", ()).await?;

    // info!("CREATE TABLE {}", );

    // Collect payments
    let (_ /* conn */, loaded_payments) = result.map_and_drop(|row| {
        let (customer_id, amount, account_name) = mysql_async::from_row(row);
        Payment {
            customer_id: customer_id,
            amount: amount,
            account_name: account_name,
        }
    }).await?;

    // The destructor of a connection will return it to the pool,
    // but pool should be disconnected explicitly because it's
    // an asynchronous procedure.
    pool.disconnect().await?;

    assert_eq!(loaded_payments, payments);

    // the async fn returns Result, so
    Ok(())
}

#[tokio::main]
async fn main() {
  env_logger::init();

  info!("Passthrough MariaDB proxy... ");
  insert().await.unwrap();
}