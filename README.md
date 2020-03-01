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
$ bash scripts/docker-enter.sh
```

## Passthrough proxy

This example just prints what's going through the proxy.

```bash
$ RUST_LOG=info cargo run --example passthrough
```
