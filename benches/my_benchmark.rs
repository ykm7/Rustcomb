extern crate dotenv;
use dotenv::from_filename;

mod file_generation;

use file_generation::{FileType, create_files};
use std::{collections::HashMap, path::Path};

use std::sync::Arc;

use assert_fs::fixture;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use rustcomb::{
    Cli, PrintDisable, async_read_files, get_cpuworkers, rayon_read_files,
    single_thread_read_files, thread_per_file_read_files, threadpool_read_files,
};

use rustcomb::my_regex::SearchMode;

fn setup(temp_dir: &fixture::TempDir) -> Arc<Cli> {
    from_filename(Path::new("benches").join(".env")).ok();

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

    let path_pattern_regex: SearchMode = envs
        .get("PATH_PATTERN_REGEX")
        .expect("Expect to find 'PATH_PATTERN_REGEX'")
        .parse::<SearchMode>()
        .unwrap();

    let path_pattern = match envs.get("PATH_PATTERN") {
        Some(v) if !v.is_empty() => Some(v),
        Some(_) => None,
        None => None,
    };

    let file_pattern: &str = envs
        .get("FILE_PATTERN")
        .expect("Expect to find 'FILE_PATTERN'");

    // let bench_print_output = envs
    //     .get("BENCH_PRINT_OUTPUT")
    //     .expect("Expect to find 'BENCH_PRINT_OUTPUT'")
    //     .parse::<bool>()
    //     .unwrap();

    let file_pattern_regex: SearchMode = envs
        .get("FILE_PATTERN_REGEX")
        .expect("Expect to find 'FILE_PATTERN_REGEX'")
        .parse::<SearchMode>()
        .unwrap();

    let file_to_duplicate: FileType = envs
        .get("FILE_TO_DUPLICATE")
        .expect("Expect to find 'FILE_TO_DUPLICATE'")
        .parse::<FileType>()
        .unwrap();

    // println!(
    //     "Parameters:\nNum of files to create: {}\nNum of directories to create: {}\nFile pattern: {}\nFile content pattern: {}\nFile type to duplicate: {}\nPrint matches: {}",
    //     num_of_files_to_create,
    //     num_of_directories_to_create,
    //     path_pattern,
    //     file_pattern,
    //     file_to_duplicate,
    //     bench_print_output
    // );

    println!("\nConfiguration:*************");
    println!(
        "Num of files to create: {}\nFile name path regex: {:?}\nFile internal regex: {}\nFile type to duplicate: {}",
        num_of_files_to_create, path_pattern, file_pattern, file_to_duplicate
    );
    println!("*****************************");

    let p = create_files(
        temp_dir,
        file_to_duplicate,
        num_of_directories_to_create,
        num_of_files_to_create,
    );

    Arc::new(Cli {
        // Initialize fields
        path_pattern: path_pattern.cloned(),
        path: p.to_path_buf(),
        file_pattern: file_pattern.to_string(),
        file_pattern_regex,
        path_pattern_regex,
    })
}

fn bench_various_reads(c: &mut Criterion) {
    let temp_dir: fixture::TempDir = assert_fs::TempDir::new().unwrap();
    let cli = setup(&temp_dir);
    let bench_print_output = PrintDisable;

    let mut group = c.benchmark_group("regex files search");

    group.bench_with_input(
        BenchmarkId::new(
            format!("single_thread_read_files_PRINT_{}", bench_print_output),
            &cli,
        ),
        &cli,
        |b, s| b.iter(|| single_thread_read_files(Arc::clone(s), bench_print_output)),
    );

    group.bench_with_input(
        BenchmarkId::new(
            format!("thread_per_file_read_files_PRINT_{}", bench_print_output),
            &cli,
        ),
        &cli,
        |b, s| b.iter(|| thread_per_file_read_files(Arc::clone(s), bench_print_output)),
    );

    group.bench_with_input(
        BenchmarkId::new(
            format!("use_thread_pool_single_thread_PRINT_{}", bench_print_output),
            &cli,
        ),
        &cli,
        |b, s| b.iter(|| threadpool_read_files(Arc::clone(s), bench_print_output, 1)),
    );

    let num_of_workers = get_cpuworkers();
    group.bench_with_input(
        BenchmarkId::new(
            format!(
                "use_thread_pool_multiple_{}_PRINT_{}",
                num_of_workers, bench_print_output
            ),
            &cli,
        ),
        &cli,
        |b, s| b.iter(|| threadpool_read_files(Arc::clone(s), bench_print_output, num_of_workers)),
    );

    group.bench_with_input(
        BenchmarkId::new(
            format!("rayon_read_files_PRINT_{}", bench_print_output),
            &cli,
        ),
        &cli,
        |b, s| b.iter(|| rayon_read_files(Arc::clone(s), bench_print_output)),
    );

    group.bench_with_input(
        BenchmarkId::new(
            format!("async_read_files_PRINT_{}", bench_print_output),
            &cli,
        ),
        &cli,
        |b, s| b.iter(|| async { async_read_files(Arc::clone(s), bench_print_output).await }),
    );

    group.finish();

    temp_dir.close().unwrap();
}

// fn benchmark_single_thread_read_files(c: &mut Criterion) {
//     let temp_dir: fixture::TempDir = assert_fs::TempDir::new().unwrap();
//     let cli = setup(&temp_dir);
//     let bench_print_output = PrintDisable;

//     c.bench_with_input(
//         BenchmarkId::new(
//             format!("single_thread_read_files_PRINT_{}", bench_print_output),
//             &cli,
//         ),
//         &cli,
//         |b, s| b.iter(|| single_thread_read_files(Arc::clone(s), bench_print_output)),
//     );

//     temp_dir.close().unwrap();
// }

// fn benchmark_thread_per_file_read_files(c: &mut Criterion) {
//     let temp_dir: fixture::TempDir = assert_fs::TempDir::new().unwrap();
//     let cli = setup(&temp_dir);
//     let bench_print_output = PrintDisable;

//     c.bench_with_input(
//         BenchmarkId::new(
//             format!("thread_per_file_read_files_PRINT_{}", bench_print_output),
//             &cli,
//         ),
//         &cli,
//         |b, s| b.iter(|| thread_per_file_read_files(Arc::clone(s), bench_print_output)),
//     );

//     temp_dir.close().unwrap();
// }

// fn benchmark_use_thread_pool_1(c: &mut Criterion) {
//     let temp_dir: fixture::TempDir = assert_fs::TempDir::new().unwrap();
//     let cli = setup(&temp_dir);
//     let bench_print_output = PrintDisable;

//     c.bench_with_input(
//         BenchmarkId::new(
//             format!("use_thread_pool_single_thread_PRINT_{}", bench_print_output),
//             &cli,
//         ),
//         &cli,
//         |b, s| b.iter(|| threadpool_read_files(Arc::clone(s), bench_print_output, 1)),
//     );

//     temp_dir.close().unwrap();
// }

// fn benchmark_use_thread_pool_multiple_num_cpus_get(c: &mut Criterion) {
//     let temp_dir: fixture::TempDir = assert_fs::TempDir::new().unwrap();
//     let cli = setup(&temp_dir);
//     let bench_print_output = PrintDisable;
//     let num_of_workers = get_cpuworkers();

//     c.bench_with_input(
//         BenchmarkId::new(
//             format!(
//                 "use_thread_pool_multiple_{}_PRINT_{}",
//                 num_of_workers, bench_print_output
//             ),
//             &cli,
//         ),
//         &cli,
//         |b, s| b.iter(|| threadpool_read_files(Arc::clone(s), bench_print_output, num_of_workers)),
//     );

//     temp_dir.close().unwrap();
// }

// fn benchmark_rayon_read_files(c: &mut Criterion) {
//     let temp_dir: fixture::TempDir = assert_fs::TempDir::new().unwrap();
//     let cli = setup(&temp_dir);
//     let bench_print_output = PrintDisable;

//     c.bench_with_input(
//         BenchmarkId::new(
//             format!("rayon_read_files_PRINT_{}", bench_print_output),
//             &cli,
//         ),
//         &cli,
//         |b, s| b.iter(|| rayon_read_files(Arc::clone(s), bench_print_output)),
//     );

//     temp_dir.close().unwrap();
// }

// fn benchmark_async_read_files(c: &mut Criterion) {
//     let temp_dir: fixture::TempDir = assert_fs::TempDir::new().unwrap();
//     let cli = setup(&temp_dir);
//     let bench_print_output = PrintDisable;

//     c.bench_with_input(
//         BenchmarkId::new(
//             format!("async_read_files_PRINT_{}", bench_print_output),
//             &cli,
//         ),
//         &cli,
//         |b, s| b.iter(|| async { async_read_files(Arc::clone(s), bench_print_output).await }),
//     );

//     temp_dir.close().unwrap();
// }

criterion_group!(benches, bench_various_reads);
criterion_main!(benches);
