extern crate dotenv;
use dotenv::from_filename;
use std::collections::HashMap;

use std::sync::Arc;

use assert_fs::fixture;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use rustcomb::file_generations::{FileType, create_files};
use rustcomb::{
    Cli, rayon_read_files, single_thread_read_files, thread_per_file_read_files,
    threadpool_read_files,
};

const FILE_TO_DUPLICATE: FileType = FileType::Light;

fn setup(temp_dir: &fixture::TempDir) -> (Arc<Cli>, bool) {
    // Perform one-time setup here
    from_filename(".\\benches\\.env").ok();

    let envs = dotenv::vars().collect::<HashMap<String, String>>();
    let num_of_files_to_create = envs
        .get("NUM_OF_FILES_TO_CREATE")
        .expect("Expect to find 'NUM_OF_FILES_TO_CREATE'")
        .parse::<usize>()
        .unwrap();

    // Not currently used
    let num_of_directories_to_create = 0;
    // let num_of_directories_to_create = envs
    //     .get("NUM_OF_DIRECTORIES_TO_CREATE")
    //     .expect("Expect to find 'NUM_OF_DIRECTORIES_TO_CREATE'")
    //     .parse::<usize>()
    //     .unwrap();

    let path_pattern: &str = envs
        .get("PATH_PATTERN")
        .expect("Expect to find 'PATH_PATTERN'");

    let file_pattern: &str = envs
        .get("FILE_PATTERN")
        .expect("Expect to find 'FILE_PATTERN'");

    let bench_print_output = envs
        .get("BENCH_PRINT_OUTPUT")
        .expect("Expect to find 'BENCH_PRINT_OUTPUT'")
        .parse::<bool>()
        .unwrap();

    let p = create_files(
        temp_dir,
        FILE_TO_DUPLICATE,
        num_of_directories_to_create,
        num_of_files_to_create,
    );

    let cli = Arc::new(Cli {
        // Initialize fields
        path_pattern: path_pattern.to_string(),
        path: p.to_path_buf(),
        file_pattern: file_pattern.to_string(),
    });

    (cli, bench_print_output)
}

fn benchmark_single_thread_read_files(c: &mut Criterion) {
    let temp_dir: fixture::TempDir = assert_fs::TempDir::new().unwrap();
    let (cli, bench_print_output) = setup(&temp_dir);

    c.bench_with_input(
        BenchmarkId::new(
            format!("single_thread_read_files_PRINT_{}", bench_print_output),
            &cli,
        ),
        &cli,
        |b, s| b.iter(|| single_thread_read_files(Arc::clone(s), bench_print_output)),
    );

    temp_dir.close().unwrap();
}

fn benchmark_thread_per_file_read_files(c: &mut Criterion) {
    let temp_dir: fixture::TempDir = assert_fs::TempDir::new().unwrap();
    let (cli, bench_print_output) = setup(&temp_dir);

    c.bench_with_input(
        BenchmarkId::new(
            format!("thread_per_file_read_files_PRINT_{}", bench_print_output),
            &cli,
        ),
        &cli,
        |b, s| b.iter(|| thread_per_file_read_files(Arc::clone(s), bench_print_output)),
    );

    temp_dir.close().unwrap();
}

fn benchmark_use_thread_pool_1(c: &mut Criterion) {
    let temp_dir: fixture::TempDir = assert_fs::TempDir::new().unwrap();
    let (cli, bench_print_output) = setup(&temp_dir);

    c.bench_with_input(
        BenchmarkId::new(
            format!("use_thread_pool_single_thread_PRINT_{}", bench_print_output),
            &cli,
        ),
        &cli,
        |b, s| b.iter(|| threadpool_read_files(Arc::clone(s), bench_print_output, 1)),
    );

    temp_dir.close().unwrap();
}

fn benchmark_use_thread_pool_num_cpus_get(c: &mut Criterion) {
    let temp_dir: fixture::TempDir = assert_fs::TempDir::new().unwrap();
    let (cli, bench_print_output) = setup(&temp_dir);

    c.bench_with_input(
        BenchmarkId::new(
            format!(
                "use_thread_pool_{}_PRINT_{}",
                num_cpus::get(),
                bench_print_output
            ),
            &cli,
        ),
        &cli,
        |b, s| b.iter(|| threadpool_read_files(Arc::clone(s), bench_print_output, num_cpus::get())),
    );

    temp_dir.close().unwrap();
}

fn benchmark_rayon_read_files(c: &mut Criterion) {
    let temp_dir: fixture::TempDir = assert_fs::TempDir::new().unwrap();
    let (cli, bench_print_output) = setup(&temp_dir);

    c.bench_with_input(
        BenchmarkId::new(
            format!("rayon_read_files_PRINT_{}", bench_print_output),
            &cli,
        ),
        &cli,
        |b, s| b.iter(|| rayon_read_files(Arc::clone(s), bench_print_output)),
    );

    temp_dir.close().unwrap();
}

criterion_group!(
    benches,
    benchmark_single_thread_read_files,
    benchmark_thread_per_file_read_files,
    benchmark_use_thread_pool_1,
    benchmark_use_thread_pool_num_cpus_get,
    benchmark_rayon_read_files
);
criterion_main!(benches);
