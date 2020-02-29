#!/bin/bash

docker network create --driver bridge devnet

docker run -it \
  --rm \
  --network devnet \
  mariadb \
  mysql --host=rust-dev --user=root --password=devpassword 
  #bash
