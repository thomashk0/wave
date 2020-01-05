#!/bin/bash

set -eu -o pipefail

cargo build
cargo build --release

make wheel
