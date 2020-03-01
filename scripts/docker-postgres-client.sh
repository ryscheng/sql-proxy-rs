#!/bin/bash

docker network create --driver bridge devnet

docker run --rm -it \
  --network devnet \
  postgres \
  psql --host rust-dev --username root
  #bash
