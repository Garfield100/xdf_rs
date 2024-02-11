#![allow(unused_imports)]

use std::env;
use std::error::Error;
use std::fs;
use xdf::*;

fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    let bytes = fs::read("/home/garfield/projects/rust/xdf-rs/example-files/tmp/xdf_001.xdf").unwrap();
    let xdf_file = XDFFile::from_bytes(&bytes).unwrap();

    println!("{:#?}", xdf_file.header);

    for stream in xdf_file.streams {
        println!(
            "{: <25} : {: >8} * {:>3} = {: >9}",
            stream.name.clone().unwrap(),
            stream.samples.len(),
            stream.channel_count,
            stream.samples.len() * stream.channel_count as usize
        );
    }
}
