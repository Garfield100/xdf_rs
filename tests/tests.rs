mod read {
    mod example_files;
    mod read_files;
    mod read_integration;
}

#[cfg(feature = "write")]
mod write {
    mod simple_num;
    mod simple_str;
}
