# A Rust-based toolkit for digital waveform manipulation

[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE.txt)
[![Build Status](https://travis-ci.com/thomashk0/wave.svg?branch=master)](https://travis-ci.com/thomashk0/wave)

The `wavetk` project is a toolkit for digital waveform parsing and processing.
The waveform we are talking about here are produced by CAD tools in formats
like [Value Change Dump (VCD)](https://en.wikipedia.org/wiki/Value_change_dump)
or FST (from [Gtkwave](http://gtkwave.sourceforge.net/)).

This project includes:

* The rust crate [wavetk](./wavetk), which contains data structures and 
  functions for parsing and processing VCD file (FST is planned)
* Low-level bindings (i.e., C-compatible) around the library are defined in [./wavetk-bindings](./wavetk-bindings))
* A Python wrapper [bindings/python](./bindings/python)

## License

This project is under a [MIT license](./LICENSE.txt).
