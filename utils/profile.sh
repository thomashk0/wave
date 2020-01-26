#!/bin/bash

valgrind --tool=callgrind \
    --dump-instr=yes \
    --collect-jumps=yes \
    --simulate-cache=yes \
    ./target/release/examples/state_simulation assets/vcd/good/verilator_riscv.vcd
