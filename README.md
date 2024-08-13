# print_raster

[![crates.io](https://img.shields.io/crates/v/print_raster.svg)](https://crates.io/crates/print_raster)
[![Released API docs](https://docs.rs/print_raster/badge.svg)](https://docs.rs/print_raster)
[![BSD 3 Clause licensed](https://img.shields.io/badge/license-BSD%203%20Clause-blue)](./LICENSE.md)

A crate for processing print raster images in Rust.

## Supported Formats
- URF (Apple Raster)
- CUPS Raster V1
- CUPS Raster V2, including PWG Raster (a subset of CUPS Raster V2)
- CUPS Raster V3

## Features
- Fully Asynchronous I/O
- Relatively low-level API

## Development
You can run unit tests, integration tests, and documentation tests with the following command:
```bash
cargo test
```

For fuzz testing, it's a bit more complicated. You need to use the `honggfuzz` tool, which only works on a few platforms. [See here](https://github.com/rust-fuzz/honggfuzz-rs) to set it up.

After setting up `honggfuzz`, you can run a fuzz target:
```bash
cargo hfuzz run <fuzz_target>
```
