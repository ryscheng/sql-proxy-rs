#!/bin/bash

docker network create --driver bridge devnet

docker run --rm -it \
  --name postgres \
  --network devnet \
  -e POSTGRES_PASSWORD=devpassword \
  -d postgres 
