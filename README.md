# sql-proxy-rs
![](https://github.com/ryscheng/sql-proxy-rs/workflows/CI/badge.svg)

Programmable Postgres/MariaDB Proxy for Rust

# Running Examples

## Setting up

You'll need an instance of either Postgres or MariaDB running. If you have Docker Compose installed, this will bring up a PostgresSQL server, MariaDB server, and a container for compiling Rust.

```bash
$ docker-compose up
```

To attach an interactive shell into the Rust development container that was launched from Docker compose.
You can attach as many shells as you'd like to run different things in the same container

``` bash
$ make shell
```

## Running tests

### Lint

From the interactive shell above, run

```bash
$ bash scripts/check.sh
```

The script will run `rustfmt`, which automatically rewrites files to match the formatting rules.
The script also runs `clippy`, which will only output errors that need to be addressed in order to pass CI

### Integration tests

We currently have integration tests that test an end-to-end passthrough proxy.
Run the following in the interactive shell (described above)

```bash
$ cargo test
```

## Passthrough proxy

This example just silently forwards packets back and forth

```bash
$ RUST_LOG=info cargo run --example passthrough -- BIND_ADDR DB_ADDR [mariadb/postgres]
# For example:
$ RUST_LOG=info cargo run --example passthrough -- 0.0.0.0:3306 mariadb-server:3306 mariadb
$ RUST_LOG=info cargo run --example passthrough -- 0.0.0.0:5432 postgres-server:5432 postgres
```

## Counter proxy

This example is the same as passthrough proxy, except it also logs any queries counts the types of queries going through (e.g. select, insert, create, etc.)

```bash
$ RUST_LOG=info cargo run --example counter -- BIND_ADDR DB_ADDR [mariadb/postgres]
# For example:
$ RUST_LOG=info cargo run --example counter -- 0.0.0.0:3306 mariadb-server:3306 mariadb
$ RUST_LOG=info cargo run --example counter -- 0.0.0.0:5432 postgres-server:5432 postgres
```

# Running a SQL client
Assuming you used the previous setup scripts to run a proxy,
you can use the following script to connect to your proxy and interactively issue SQL commands

```bash
$ make mysql    # client to a MariaDB proxy
OR 
$ make psql     # client to a Postgres proxy
```

