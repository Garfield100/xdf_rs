# xdf_rs

Rust crate for parsing (and maybe one day writing) XDF files.
Currently the only supported XDF version is 1.0. (at the time of writing, this the only version that exists)

[XDF format specification by SCCN](https://github.com/sccn/xdf/wiki/Specifications)

## Installation

`cargo add xdf`

## Example usage

```rust
use std::fs;
use xdf::XDFFile;
let bytes = fs::read("tests/minimal.xdf").unwrap();
let xdf_file = XDFFile::from_bytes(&bytes).unwrap();
```
