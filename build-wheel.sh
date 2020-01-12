#!/bin/bash

set -eu -o pipefail

pydir=bindings/python

cargo build
cp -v target/debug/libwavetk_bindings.so ${pydir}/wavetk/libwavetk_bindings_debug.so

cargo build --release
cp -v target/release/libwavetk_bindings.so ${pydir}/libwavetk_bindings.so

(cd bindings/python && python setup.py bdist_wheel)
