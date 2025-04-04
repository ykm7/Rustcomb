# Rustcomb

This project is another Grep style program written is Rust.

Purpose of this is primarily to function as a learning ground for Rust.
Output format taken from: [ripgrep](https://github.com/BurntSushi/ripgrep) as this appears to be the most prominent Rust CLI search tool.

Therefore currently there are several implementations of grep like function running on serial using a variety of:
* single/multiple threads
* threadpools (which varied "CPUs")
* Popular threading library Rayon.
* Async (Tokio)

NOTE: The current project is not the final product. Given the nature of the project I will be making constant tweaks to improve the UX as well as attempting to correct/improve basic coding issues.

## Example screenshots:

No path filter specified
[![Example no path filter specified:](/docs/images/runtime_no_path_filer_literal.png)](/docs/images/runtime_no_path_filer_literal.png)

Regex applied to both file names as well as internal file search. These can be mixed and matched
[![Example no path filter specified:](/docs/images/runtime_regex.png)](/docs/images/runtime_regex.png)

## CLI

https://crates.io/crates/wild

General 
> cargo run [FILE_NAME_PATH_REGEX] [FILE_OR_DIRECTORY_TO_EXAMINE] [REGEX_WITHIN_FILE_TO_FIND]

> cargo run --release [FILE_NAME_PATH_REGEX] [FILE_OR_DIRECTORY_TO_EXAMINE] [REGEX_WITHIN_FILE_TO_FIND]

## Testing

> cargo test

## Async
Async implemented is performed with Tokio.
~~Overall, there is a limited benefit for implementing this.
Async benefits for IO usage, however this generally excludes file reading as the OS's underlining
API does not take advantage of async and is usually blocking.
https://tokio.rs/tokio/tutorial: "When not to use Tokio".~~

based on benchmark results even with the most default implementation I seem to be seeing ns performance instead of ms when incorporating Tokio. Need further testing/investigation.

Therefore this is primarily to provide some familiarity with Async Rust
(not a new concept overall, as familiar with Async with webbased Typescript usage)

## Benchmarking - [Criterion](https://bheisler.github.io/criterion.rs)
As part of my continued understanding of how Rust operations I have established benchmarks of all file retrieving and parsing

A environment file should be supplied within the `benches` directory.

```.env
// Number of files to duplicate
NUM_OF_FILES_TO_CREATE=10
// NOTE! Not currently used.
NUM_OF_DIRECTORIES_TO_CREATE=0
// [OPTIONAL] the regex pattern to filter the files.
PATH_PATTERN=".txt"
// [OPTIONAL] Required to be "literal" or "regex" - defaults to "literal"
PATH_PATTERN_REGEX="literal"
// the regex pattern to find within the files.
FILE_PATTERN="cubilia"
// [OPTIONAL] Required to be "literal" or "regex" - defaults to "literal"
FILE_PATTERN_REGEX="literal"
// NOTE! Not currently used - Enable printing of program output
BENCH_PRINT_OUTPUT=false
// Required to be "light", "medium" or "heavy" (case-insensitive) 
// This reflects the file type to be genererated in bulk per to the benchmark running against the files.
FILE_TO_DUPLICATE=light
```

Example screenshot:
[![Example screenshot:](/docs/images/benchmark_basic_async_inclusion.png)](/docs/images/benchmark_1.png)

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

### CPU

Perf can be used with the below setup for WSL.

Example:
> perf record cargo run --release *.txt test_files vel

Or with benchmarks

> perf record cargo bench --bench my_benchmark use_thread_pool_multiple

View output
> perf report

For all the benchmarks for comparision

    perf record -o single_thread_read_files.data cargo bench --bench my_benchmark single_thread_read_files &&
    perf record -o thread_per_file_read_files.data cargo bench --bench my_benchmark thread_per_file_read_files &&
    perf record -o use_thread_pool_single_thread.data cargo bench --bench my_benchmark use_thread_pool_single_thread &&
    perf record -o use_thread_pool_multiple.data cargo bench --bench my_benchmark use_thread_pool_multiple &&
    perf record -o rayon_read_files.data cargo bench --bench my_benchmark rayon_read_files


### Memory (TODO)

## Linting (Clippy)

Clippy is [used](https://github.com/rust-lang/rust-clippy) to try to pick up addition issues/suggestions. Very handy while learning.

## Misc

### Crates

[machete](https://crates.io/crates/cargo-machete/)

## OS

My current development environment is Windows, however using WSL (Ubuntu) to access linux profiling tools.

### Ubuntu (WSL)

> sudo apt  install rustup

> rustup default stable 

For a C linker
> sudo apt install build-essential

(Provide the `.env` within the benches directory)

#### perf

> sudo apt install linux-tools-common

WSL error

    WARNING: perf not found for kernel 5.15.167.4-microsoft

    You may need to install the following packages for this specific kernel:
        linux-tools-5.15.167.4-microsoft-standard-WSL2
        linux-cloud-tools-5.15.167.4-microsoft-standard-WSL2

    You may also want to install one of the following packages to keep up to date:
        linux-tools-standard-WSL2
        linux-cloud-tools-standard-WSL2

> sudo apt install linux-tools-generic

WSL2 uses a custom kernel so have to build our own `perf`.
Although from what I am reading it makes sense that running perf within a VM may be limited.

> https://stackoverflow.com/questions/60237123/is-there-any-method-to-run-perf-under-wsl

Downloading the version manually from the above link.

> wget "https://mirrors.edge.kernel.org/pub/linux/kernel/tools/perf/v6.9.0/perf-6.9.0.tar.gz"

> tar xzf perf-6.9.0.tar.gz

> cd linux-6.9/tools/perf

> sudo apt install clang libcapstone-dev libtraceevent-dev libtracefs-dev

> make -j$(nproc)

Add the path shell config file (note, added to the beginning of the path is there is a default version which results in the above errors)
Adding:

`echo 'export PATH="$HOME/repos/tools/perf-6.9.0/tools/perf:$PATH"' >> ~/.bashrc`