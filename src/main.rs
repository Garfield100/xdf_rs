#![allow(unused_imports)]

use std::env;
use std::error::Error;
use std::fs;
use xdf::*;

fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    // println!("{}",std::env::current_dir().unwrap());
    let reader = fs::File::open("/home/garfield/projects/rust/xdf-rs/example-files/minimal.xdf").unwrap();
    let raw_chunks = read_to_raw_chunks(reader).unwrap();
    // println!("{:#?}\n{}",chunks, chunks.len());

    // for raw_chunk in &raw_chunks {
    //     println!("{:?}", raw_chunk.tag);
    // }

    let chunks = match raw_chunks_to_chunks(raw_chunks) {
        Ok(res) => res,
        Err(err) => {
            println!("Error: {:?}", &err);
            println!("Source: {:?}", &err.source());

            panic!("Encountered an error while converting raw chunks to chunks. See above.");
        }
    };
    for chunk in chunks {
        println!("{:?}", chunk);
    }
}
