use criterion::{Criterion, black_box, criterion_group, criterion_main};

use rustcomb::{
    Cli, rayon_read_files, single_thread_read_files, thread_per_file_read_files,
    threadpool_read_files,
};

fn benchmark_single_thread_read_files(c: &mut Criterion) {
    let cli = Cli {
        path_pattern: ".txt".to_string(),
        path: ".\\test_file_direction\\".into(),
        file_pattern: "test".to_string(),
    };

    c.bench_function(
        format!("'single_thread_read_files': {:?}\n", cli).as_str(),
        |b| {
            b.iter(|| {
                let cli = Cli {
                    path_pattern: ".txt".to_string(),
                    path: ".\\test_file_direction\\".into(),
                    file_pattern: "test".to_string(),
                };
                single_thread_read_files(black_box(cli))
            })
        },
    );
}

fn benchmark_thread_per_file_read_files(c: &mut Criterion) {
    let cli = Cli {
        path_pattern: ".txt".to_string(),
        path: ".\\test_file_direction\\".into(),
        file_pattern: "test".to_string(),
    };

    c.bench_function(
        format!("'thread_per_file_read_files': {:?}\n", cli).as_str(),
        |b| {
            b.iter(|| {
                let cli = Cli {
                    path_pattern: ".txt".to_string(),
                    path: ".\\test_file_direction\\".into(),
                    file_pattern: "test".to_string(),
                };
                thread_per_file_read_files(black_box(cli))
            })
        },
    );
}

fn benchmark_use_thread_pool_1(c: &mut Criterion) {
    let cli = Cli {
        path_pattern: ".txt".to_string(),
        path: ".\\test_file_direction\\".into(),
        file_pattern: "test".to_string(),
    };

    c.bench_function(
        format!("'use_thread_pool - 1 thread': {:?}\n", cli).as_str(),
        |b| {
            b.iter(|| {
                let cli = Cli {
                    path_pattern: ".txt".to_string(),
                    path: ".\\test_file_direction\\".into(),
                    file_pattern: "test".to_string(),
                };
                threadpool_read_files(black_box(cli), 1)
            })
        },
    );
}

fn benchmark_use_thread_pool_num_cpus_get(c: &mut Criterion) {
    let cli = Cli {
        path_pattern: ".txt".to_string(),
        path: ".\\test_file_direction\\".into(),
        file_pattern: "test".to_string(),
    };

    c.bench_function(
        format!("'use_thread_pool - {}': {:?}\n", num_cpus::get(), cli).as_str(),
        |b| {
            b.iter(|| {
                let cli = Cli {
                    path_pattern: ".txt".to_string(),
                    path: ".\\test_file_direction\\".into(),
                    file_pattern: "test".to_string(),
                };
                threadpool_read_files(black_box(cli), num_cpus::get())
            })
        },
    );
}

fn benchmark_rayon_read_files(c: &mut Criterion) {
    let cli = Cli {
        path_pattern: ".txt".to_string(),
        path: ".\\test_file_direction\\".into(),
        file_pattern: "test".to_string(),
    };

    c.bench_function(format!("'rayon_read_files': {:?}\n", cli).as_str(), |b| {
        b.iter(|| {
            let cli = Cli {
                path_pattern: ".txt".to_string(),
                path: ".\\test_file_direction\\".into(),
                file_pattern: "test".to_string(),
            };
            rayon_read_files(black_box(cli))
        })
    });
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
