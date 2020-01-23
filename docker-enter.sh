#!/bin/bash

docker run --rm -it \
  --name rust-dev \
  -v "$PWD":/code \
  rust \
  bash

