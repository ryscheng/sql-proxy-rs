#!/bin/bash

docker network create --driver bridge devnet

docker run --rm -it \
  --name proxy \
  --network devnet \
  -v "$PWD":/code \
  -p 3306:3306 \
  -w /code \
  rust \
  bash

