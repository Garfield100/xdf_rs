use glassbench::*;
use xdf;

fn bench_parse_files(bench: &mut Bench) {
    // load the file into memory
    // let xdf_001_bytes = std::fs::read("example-files/tmp/xdf_001.xdf").unwrap();
    // bench.task("xdf_001.xdf - 592M", |task| {
    //     task.iter(|| {
    //         let xdf_data = xdf::XDFFile::from_reader(xdf_001_bytes.as_slice()).unwrap();
    //         pretend_used(&xdf_data);
    //     });
    // });

    let minimal_bytes = std::fs::read("example-files/minimal.xdf").unwrap();
    bench.task("minimal.xdf - 4.0K", |task| {
        task.iter(|| {
            let xdf_data = xdf::XDFFile::from_bytes(minimal_bytes.as_slice()).unwrap();
            pretend_used(&xdf_data);
        });
    });

    let xdf_009_bytes = std::fs::read("example-files/tmp/xdf_009.xdf").unwrap();
    bench.task("xdf_009.xdf - 7.5M", |task| {
        task.iter(|| {
            let xdf_data = xdf::XDFFile::from_bytes(xdf_009_bytes.as_slice()).unwrap();
            pretend_used(&xdf_data);
        });
    });

}

glassbench!("parse_files", bench_parse_files,);
