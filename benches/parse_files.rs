use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs;

// Files used in the tmp folder here can be downloaded from https://osf.io/uc7wn/ (thank you to Clemens Brunner for the upload)

fn bench_parse_files(c: &mut Criterion) {
    let minimal_bytes = fs::read("tests/minimal.xdf").unwrap();
    c.bench_function("minimal.xdf - 4.0K", |b| {
        b.iter(|| {
            let xdf_data = xdf::XDFFile::from_bytes(black_box(&minimal_bytes)).unwrap();
            black_box(xdf_data);
        });
    });

    // benchmarks for bigger files which can be downloaded from the link above.
    // They are much less consistent across trials than the small one, likely for scheduling and i/o reasons.

    // let xdf_009_bytes = fs::read("example-files/tmp/xdf_009.xdf").unwrap();
    // c.bench_function("xdf_009.xdf - 7.5M", |b| {
    //     b.iter(|| {
    //         let xdf_data = xdf::XDFFile::from_bytes(black_box(&xdf_009_bytes)).unwrap();
    //         black_box(xdf_data);
    //     });
    // });

    // let xdf_006_bytes = fs::read("example-files/tmp/xdf_006.xdf").unwrap();
    // c.bench_function("xdf_006.xdf - 62M", |b| {
    //     b.iter(|| {
    //         let xdf_data = xdf::XDFFile::from_bytes(black_box(&xdf_006_bytes)).unwrap();
    //         black_box(xdf_data);
    //     });
    // });

    // let xdf_001_bytes = fs::read("example-files/tmp/xdf_001.xdf").unwrap();
    // c.bench_function("xdf_001.xdf - 592M", |b| {
    //     b.iter(|| {
    //         let xdf_data = xdf::XDFFile::from_bytes(black_box(&xdf_001_bytes)).unwrap();
    //         black_box(xdf_data);
    //     });
    // });
}

criterion_group! {
    name = benches;
    // This can be any expression that returns a `Criterion` object.
    config = Criterion::default().significance_level(0.1).sample_size(500);
    targets = bench_parse_files
}
criterion_main!(benches);
