#!/bin/bash

docker network create --driver bridge devnet

docker run --rm -it \
  --network devnet \
  -v "/tmp/tendermint:/tendermint" \
  --name tendermint \
  tendermint/tendermint:v0.32.8 \
  init

docker run --rm -it \
  --network devnet \
  -v "/tmp/tendermint:/tendermint" \
  --name tendermint \
  tendermint/tendermint:v0.32.8 \
  unsafe_reset_all
  
docker run --rm -it \
  --network devnet \
  -v "/tmp/tendermint:/tendermint" \
  --name tendermint \
  tendermint/tendermint:v0.32.8 \
  node --proxy_app=tcp://rust-dev:26658 --consensus.create_empty_blocks=false --rpc.laddr=tcp://0.0.0.0:26657
  
