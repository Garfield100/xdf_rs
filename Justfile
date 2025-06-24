test:
    cargo test

mutate:
    @cargo mutants --jobs 10 --timeout 2

bench:
    cargo bench

flamegraph:
    cargo flamegraph -F 10000 --no-inline --profile profiler --bench parse_files -- --bench

fix:
    cargo fmt
    cargo fix --allow-dirty
    cargo clippy --fix --allow-dirty

coverage:
    cargo tarpaulin --out html