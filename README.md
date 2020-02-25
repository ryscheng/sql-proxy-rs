# mariadb-proxy-rs
Programmable MariaDB Proxy for Rust


# Running Examples

## Setting up

You'll need an instance of MariaDB running. If you have Docker installed, there are some convenience scripts

```bash
$ bash scripts/docker-mariadb.sh  # Will start a MariaDB container in the background
$ bash scripts/docker-enter.sh    # Will open an interactive shell into a Rust development container on the same network
```

## Passthrough proxy

This example just prints what's going through the proxy.

```bash
$ RUST_LOG=info cargo run --example passthrough
```
