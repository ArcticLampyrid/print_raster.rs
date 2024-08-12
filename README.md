# print_raster
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
