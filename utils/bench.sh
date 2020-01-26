#!/bin/bash

set -e

# Put some large VCD files there
vcd_dir="../vcd-large"

for f in $(ls --sort=size  ${vcd_dir}/*.vcd); do
    echo -e -n "$f\t$(stat --printf="%s" $f)\t"
    perf stat ./target/release/examples/state_simulation $f 2>&1 | grep elapsed | awk '// {print $1}'
done
