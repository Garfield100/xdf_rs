#![allow(unused_imports)]

use std::fs;
use std::env;
use xdf::*;

fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    // println!("{}",std::env::current_dir().unwrap());
    let raw_chunks = read_file_to_raw_chunks("/home/garfield/projects/rust/xdf-rs/example-files/minimal.xdf").unwrap();
    // println!("{:#?}\n{}",chunks, chunks.len());

    for raw_chunk in raw_chunks {
        println!("{:?}", raw_chunk.tag);
        println!("{:#?}", raw_chunk_to_chunk::<u8>(raw_chunk));
    }
}   