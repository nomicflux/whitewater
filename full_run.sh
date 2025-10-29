#!/bin/bash

set -e

cargo build
# ./test_run.sh || echo "Done"

docker build -t whitewater:2 .
./reset_kind.sh
