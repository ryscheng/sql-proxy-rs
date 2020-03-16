#!/bin/bash

# Only using nightly for tooling (e.g. rustfmt, clippy)!
# Source code should stay on stable for now
rustup toolchain install nightly
rustup component add rustfmt --toolchain nightly
cargo +nightly fmt

rustup component add clippy
cargo clippy

