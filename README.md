# Rustcomb

This project is another Grep style program written is Rust.

Purpose of this is primarily to function as a learning ground for Rust.

Therefore currently there are several implementations of grep like function running on serial using a variety of:
* single/multiple threads
* threadpools (which varied "CPUs")
* Popular threading library Rayon.
* TODO Async 

NOTE: The current project is not the final product. Given the nature of the project I will be making constant tweaks to improve the UX as well as attempting to correct/improve basic coding issues.

## CLI

https://crates.io/crates/wild

General 
> cargo run [FILE_NAME_PATH_REGEX] [FILE_OR_DIRECTORY_TO_EXAMINE] [REGEX_WITHIN_FILE_TO_FIND]

> cargo run --release [FILE_NAME_PATH_REGEX] [FILE_OR_DIRECTORY_TO_EXAMINE] [REGEX_WITHIN_FILE_TO_FIND]

## Testing

> cargo test

## Benchmarking - [Criterion](https://bheisler.github.io/criterion.rs)
As part of my continued understanding of how Rust operations I have established benchmarks of all file retrieving and parsing

A environment file should be supplied within the `benches` directory.

```.env
// Number of files to duplicate
NUM_OF_FILES_TO_CREATE=10
// Not currently used.
NUM_OF_DIRECTORIES_TO_CREATE=0
// the regex pattern to filter the files.
PATH_PATTERN=".txt"
// the regex pattern to find within the files.
FILE_PATTERN="cubilia"
// Enable printing of program output
BENCH_PRINT_OUTPUT=false
// Required to be "light", "medium" or "heavy" (case-insensitive) 
// This reflects the file type to be genererated in bulk per to the benchmark running against the files.
FILE_TO_DUPLICATE=light
```

### File generation
Generated with: [Lorem Ipsum Generator](https://loremipsum.io/generator?n=10&t=p)

#### Light
10x paragraphs

#### Medium
100x paragraphs

#### Heavy
1000x paragraphs

Benchmark all
> cargo bench

Available manual benches to benchmark particular one

> cargo bench --bench my_benchmark single_thread_read_files

> cargo bench --bench my_benchmark thread_per_file_read_files

> cargo bench --bench my_benchmark use_thread_pool_single_thread

> cargo bench --bench my_benchmark use_thread_pool_multiple

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