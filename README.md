# Rustcomb

## CLI

https://crates.io/crates/wild

General 
> cargo run .txt .\test_file_direction\ test

> cargo run --release .txt .\test_file_direction\ test

## Benchmarking
As part of my continued understanding of how Rust operations I have established benchmarks of all file retrieving and parsing

Benchmark all
> cargo bench

Benchmark particular one
> cargo bench --bench my_benchmark rayon_read_files

_!Note_ the lack of "benchmark_" on the benchmark function name.

### CPU (TODO)

<!-- > perf record `target\release\rustcomb.exe .txt .\test_file_direction\ test` -- --profile-time 10

TODO: Require WSL to be running this "locally". -->

### Memory

## Linting (Clippy)

Clippy is [used](https://github.com/rust-lang/rust-clippy) to try to pick up addition issues/suggestions. Very handy while learning.