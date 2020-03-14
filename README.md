# mariadb-proxy-rs
Programmable Postgres/MariaDB Proxy for Rust


# Running Examples

## Setting up

You'll need an instance of either Postgres or MariaDB running. If you have Docker installed, there are some convenience scripts

```bash
$ bash scripts/docker-mariadb-server.sh    # Will start a MariaDB container in the background
# OR
$ bash scripts/docker-postgres-server.sh   # Will start a Postgres container in the background
```

To open an interactive shell into a Rust development container on the same network:

``` bash
$ bash scripts/docker-enter.sh    # Will open an interactive shell into a Rust development container on the same network as MariaDB
```


## Passthrough proxy

This example just silently forwards packets back and forth

```bash
$ RUST_LOG=info cargo run --example passthrough
```

## Counter proxy

This example is the same as passthrough proxy, except it also logs any queries counts the types of queries going through (e.g. select, insert, create, etc.)

```bash
$ RUST_LOG=info cargo run --example counter 
```

## Tendermint proxy

This example forwards all queries to a Tendermint network, replicating the query

```bash
$ RUST_LOG=info cargo run --example tendermint
```

# Running a SQL client
Assuming you used the previous setup scripts to run a proxy and MariaDB instance,
you can use the following script to connect to your proxy and issue SQL commands

```bash
$ bash scripts/docker-sqlclient.sh
```
