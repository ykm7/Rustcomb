use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use rustcomb::{
    Cli, rayon_read_files, single_thread_read_files, thread_per_file_read_files,
    threadpool_read_files,
};

lazy_static::lazy_static! {
    static ref CLI_ARGS: Arc<Cli> = Arc::new(Cli {
        path_pattern: ".txt".to_string(),
        path: ".\\test_file_direction\\".into(),
        file_pattern: "asdfasdftest".to_string(),
    });
}

const PRINT: bool = false;

fn benchmark_single_thread_read_files(c: &mut Criterion) {
    let cli = Arc::clone(&CLI_ARGS);
    c.bench_with_input(
        BenchmarkId::new(format!("single_thread_read_files_PRINT_{}", PRINT), &cli),
        &cli,
        |b, s| b.iter(|| single_thread_read_files(Arc::clone(s), PRINT)),
    );
    c.bench_with_input(
        BenchmarkId::new(format!("single_thread_read_files_PRINT_{}", !PRINT), &cli),
        &cli,
        |b, s| b.iter(|| single_thread_read_files(Arc::clone(s), !PRINT)),
    );
}

fn benchmark_thread_per_file_read_files(c: &mut Criterion) {
    let cli = Arc::clone(&CLI_ARGS);
    c.bench_with_input(
        BenchmarkId::new(format!("thread_per_file_read_files_PRINT_{}", PRINT), &cli),
        &cli,
        |b, s| b.iter(|| thread_per_file_read_files(Arc::clone(s), PRINT)),
    );
    c.bench_with_input(
        BenchmarkId::new(format!("thread_per_file_read_files_PRINT_{}", !PRINT), &cli),
        &cli,
        |b, s| b.iter(|| thread_per_file_read_files(Arc::clone(s), !PRINT)),
    );
}

fn benchmark_use_thread_pool_1(c: &mut Criterion) {
    let cli = Arc::clone(&CLI_ARGS);
    c.bench_with_input(
        BenchmarkId::new(
            format!("use_thread_pool_single_thread_PRINT_{}", PRINT),
            &cli,
        ),
        &cli,
        |b, s| b.iter(|| threadpool_read_files(Arc::clone(s), PRINT, 1)),
    );

    c.bench_with_input(
        BenchmarkId::new(
            format!("use_thread_pool_single_thread_PRINT_{}", !PRINT),
            &cli,
        ),
        &cli,
        |b, s| b.iter(|| threadpool_read_files(Arc::clone(s), !PRINT, 1)),
    );
}

fn benchmark_use_thread_pool_num_cpus_get(c: &mut Criterion) {
    let cli = Arc::clone(&CLI_ARGS);
    c.bench_with_input(
        BenchmarkId::new(
            format!("use_thread_pool_{}_PRINT_{}", num_cpus::get(), PRINT),
            &cli,
        ),
        &cli,
        |b, s| b.iter(|| threadpool_read_files(Arc::clone(s), PRINT, num_cpus::get())),
    );

    c.bench_with_input(
        BenchmarkId::new(
            format!("use_thread_pool_{}_PRINT_{}", num_cpus::get(), !PRINT),
            &cli,
        ),
        &cli,
        |b, s| b.iter(|| threadpool_read_files(Arc::clone(s), !PRINT, num_cpus::get())),
    );
}

fn benchmark_rayon_read_files(c: &mut Criterion) {
    let cli = Arc::clone(&CLI_ARGS);
    c.bench_with_input(
        BenchmarkId::new(format!("rayon_read_files_PRINT_{}", PRINT), &cli),
        &cli,
        |b, s| b.iter(|| rayon_read_files(Arc::clone(s), PRINT)),
    );

    c.bench_with_input(
        BenchmarkId::new(format!("rayon_read_files_PRINT_{}", !PRINT), &cli),
        &cli,
        |b, s| b.iter(|| rayon_read_files(Arc::clone(s), !PRINT)),
    );
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
