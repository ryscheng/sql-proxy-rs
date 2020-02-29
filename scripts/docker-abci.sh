#!/bin/bash

docker network create --driver bridge devnet

docker run --rm -it \
  --name abci \
  --network devnet \
  -v "$PWD":/code \
  -w /code \
  rust