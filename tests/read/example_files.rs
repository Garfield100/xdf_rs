use std::fs;

use xdf::XDFFile;

#[test]
/// check if there is a folder with example files at "example-files/tmp" and check for xdf files to test. We parse them without checking the output just to check that we don't panic on any of them.
fn ensure_no_panic() {
    let path = "example-files/tmp";
    if let Ok(dir) = fs::read_dir(path) {
        let xdf_paths: Vec<_> = dir
            .filter_map(Result::ok)
            .filter_map(|entry| {
                // we only want small xdf files. 2^24 = 16 MiB
                if entry.file_name().into_string().unwrap().ends_with(".xdf")
                    && entry.metadata().unwrap().len() < 2_u64.pow(24)
                {
                    Some(entry.path())
                } else {
                    None
                }
            })
            .collect();

        println!("Reading {} files", xdf_paths.len());
        for file_path in xdf_paths {
            print!("Reading: {: >30}", file_path.file_name().unwrap().to_string_lossy());
            let bytes = fs::read(file_path).unwrap();

            // we don't unwrap because there are also examples of corrupt ones
            let xdf_file = XDFFile::from_bytes(&bytes);
            println!(
                "... {}",
                if xdf_file.is_ok() {
                    "Ok".to_string()
                } else {
                    format!("{xdf_file:?}")
                }
            )
        }
    } else {
        println!("No example files found at {path}, skipping");
    }
}
