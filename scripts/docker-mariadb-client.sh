#!/bin/bash

docker network create --driver bridge devnet

docker run --rm -it \
  --network devnet \
  mariadb \
  mysql --host=rust-dev --user=root --password=devpassword 
  #bash
