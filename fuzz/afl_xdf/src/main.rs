use afl::fuzz;
use xdf::XDFFile;

fn main() {
    fuzz! {|data: &[u8]|{
        let _ = XDFFile::from_bytes(data);
    }
    }
}
