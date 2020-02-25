#!/bin/bash

docker run --rm -it \
  --name rust-dev \
  -v "$PWD":/code \
  -p 3306:3306 \
  -w /code \
  rust \
  bash

