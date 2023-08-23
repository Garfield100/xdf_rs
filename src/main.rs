#![allow(unused_imports)]

use std::env;
use std::error::Error;
use std::fs;
use xdf::*;

fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    // println!("{}",std::env::current_dir().unwrap());
    // let reader = fs::File::open("/home/garfield/projects/rust/xdf-rs/example-files/minimal.xdf").unwrap();
    let reader = fs::File::open("/home/garfield/projects/rust/xdf-rs/example-files/minimal.xdf").unwrap();
    let xdf_file = XDFFile::from_reader(reader).unwrap();

    println!("{:?}", xdf_file.streams);
}
