# Rustcomb

This project is another Grep style program written is Rust.

Purpose of this is primarily to function as a learning ground for Rust.

NOTE: The currect project is not the final product. Given the nature of the project I will be making constant tweaks to improve the UX as well as attempting to correct/improve basic coding issues.

## CLI

https://crates.io/crates/wild

General 
> cargo run .txt .\test_file_direction\ test

> cargo run --release .txt .\test_file_direction\ test

## Testing

> cargo test

## Benchmarking
As part of my continued understanding of how Rust operations I have established benchmarks of all file retrieving and parsing

Benchmark all
> cargo bench

Benchmark particular one
> cargo bench --bench my_benchmark rayon_read_files

_!Note_ the lack of "benchmark_" on the benchmark function name.

### Test/Bench resources
Several hundred duplicate files are provided with inclusions of the "test" field for the purpose of the above testing and/or benchmarking.

### CPU (TODO)

<!-- > perf record `target\release\rustcomb.exe .txt .\test_file_direction\ test` -- --profile-time 10

TODO: Require WSL to be running this "locally". -->

### Memory (TODO)

## Linting (Clippy)

Clippy is [used](https://github.com/rust-lang/rust-clippy) to try to pick up addition issues/suggestions. Very handy while learning.