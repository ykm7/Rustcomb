use criterion::{Criterion, black_box, criterion_group, criterion_main};

use rustcomb::{Cli, single_thread_read_files,rayon_read_files};

fn criterion_benchmark(c: &mut Criterion) {
    let cli = Cli {
        path_pattern: ".txt".to_string(),
        path: ".\\test_file_direction\\".into(),
        file_pattern: "test".to_string()
    };
    
    c.bench_function(
        format!("'single_thread_read_files': {:?}\n", cli).as_str(),
        |b| {
            b.iter(|| {
                let cli = Cli {
                    path_pattern: ".txt".to_string(),
                    path: ".\\test_file_direction\\".into(),
                    file_pattern: "test".to_string()
                };
                single_thread_read_files(black_box(cli))
            })
        },
    );

    c.bench_function(
        format!("'rayon_read_files': {:?}\n", cli).as_str(),
        |b| {
            b.iter(|| {
                let cli = Cli {
                    path_pattern: ".txt".to_string(),
                    path: ".\\test_file_direction\\".into(),
                    file_pattern: "test".to_string()
                };
                rayon_read_files(black_box(cli))
            })
        },
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
