// #[macro_use]
// extern crate log;

// use env_logger;
// use mysql::*;
// use mysql::prelude::*;

// #[derive(Debug, PartialEq, Eq, Clone)]
// struct Payment {
//     customer_id: i32,
//     amount: i32,
//     account_name: Option<String>,
// }

// #[test]
// fn can_proxy_requests_to_tendermint() -> Result<()> {
//     let database_uri = "mysql://root:devpassword@mariadb-proxy:3306/testdb";
//     let pool = Pool::new(database_uri).unwrap();
//     let mut conn = pool.get_conn().unwrap();
    
//     conn.query_drop(
//         r"create table payment (customer_id int not null, amount int not null, account_name text)")?;

//     // Get the initial block height
//     let initial_block_height: Option<i32> = conn
//             .query_first(r"select max(block_height) from tendermint_blocks")
//             .unwrap();

//     let payments = vec![
//         Payment { customer_id: 1, amount: 2, account_name: None },
//         Payment { customer_id: 3, amount: 4, account_name: Some("foo".into()) },
//         Payment { customer_id: 5, amount: 6, account_name: None },
//         Payment { customer_id: 7, amount: 8, account_name: None },
//         Payment { customer_id: 9, amount: 10, account_name: Some("bar".into()) },
//     ];

//     // Insert data 
//     conn.exec_batch(
//         r"insert into payment (customer_id, amount, account_name) VALUES (:customer_id, :amount, :account_name)",
//         payments.iter().map(|p| params! {
//             "customer_id" => p.customer_id,
//             "amount" => p.amount,
//             "account_name" => &p.account_name,
//         })
//     )?;

//     // Grab data from db
//     let selected_payments = conn
//         .query_map(
//             "SELECT customer_id, amount, account_name from payment",
//             |(customer_id, amount, account_name)| {
//                 Payment { customer_id, amount, account_name }
//             },
//         )?;

//     // Assert data is correct
//     assert_eq!(payments, selected_payments);

//     let final_block_height: Option<i32> = conn
//         .query_first(r"select max(block_height) from tendermint_blocks")
//         .unwrap();
    
//     assert_eq!(final_block_height.unwrap(), initial_block_height.unwrap() + 2);

//     Ok(())
// }