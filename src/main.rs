#![allow(unused_imports)]

use std::fs;
use std::env;
use xdf::*;

fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    // println!("{}",std::env::current_dir().unwrap());
    let chunks = read_file_to_chunks("/home/garfield/projects/rust/xdf-rs/example-files/minimal.xdf");
    println!("{:#?}",chunks);
}   