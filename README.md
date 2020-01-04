# A Rust-based toolkit for digital waveform manipulation

The `wave` project is a toolkit for digital waveform parsing and processing.
The waveform we are talking about here are produced by CAD tools in formats
like [Value Change Dump (VCD)](https://en.wikipedia.org/wiki/Value_change_dump)
or FST (from [Gtkwave](http://gtkwave.sourceforge.net/)).

This project includes:

* A Rust crate called `wave`, which contains structure functions for parsing and processing VCD file (FST is planned)
* Low-level bindings to call these functions from other languages (see [./bindings](./bindings))
* Python wrapper built on top of the FFI bindings

## License

This project is under a MIT license.
