use clap::Parser;
use std::{error::Error, time::Instant};
use wild::args_os;

fn setup(args: rustcomb::Cli, print: bool) -> Result<(), Box<dyn Error>> {
    println!("Args: {:?}", args);

    // let mut matched_paths: Vec<FileInfo> = Vec::new();

    let args_clone = args.clone();
    let start = Instant::now();
    rustcomb::single_thread_read_files(args_clone, print)?;
    println!(
        "Time taken for identifying files (single_thread_read_files): {:?}",
        start.elapsed()
    );

    println!();
    println!();

    let args_clone = args.clone();
    let start = Instant::now();
    rustcomb::thread_per_file_read_files(args_clone, print)?;
    println!(
        "Time taken for identifying files (use_thread_per_file): {:?}",
        start.elapsed()
    );

    println!();
    println!();

    let args_clone = args.clone();
    let start = Instant::now();
    rustcomb::threadpool_read_files(args_clone, print, 1)?;
    println!(
        "Time taken for identifying files (use_thread_pool - 1 thread): {:?}",
        start.elapsed()
    );

    println!();
    println!();

    let args_clone = args.clone();
    let cpus = num_cpus::get();
    let start = Instant::now();
    rustcomb::threadpool_read_files(args_clone, print, cpus)?;
    println!(
        "Time taken for identifying files (use_thread_pool - {} thread): {:?}",
        cpus,
        start.elapsed()
    );

    println!();
    println!();

    let args_clone = args.clone();
    let start = Instant::now();
    rustcomb::rayon_read_files(args_clone, print)?;
    println!(
        "Time taken for identifying files (rayon_read_files): {:?}",
        start.elapsed()
    );

    Ok(())
}

fn main() {
    let cli = rustcomb::Cli::parse_from(args_os());
    if let Err(e) = setup(cli, true) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use assert_cmd::Command;
    // use predicates::prelude::*;

    #[test]
    fn test_setup() {
        let args = vec!["Rustcomb", "*.txt", ".", "hello"];
        let cli = rustcomb::Cli::parse_from(args);
        // Use setup_with_args instead of setup to pass custom arguments
        assert!(setup(cli, true).is_ok());
    }

    // #[test]
    // fn test_run_main() {
    //     let mut cmd = Command::cargo_bin("Rustcomb").unwrap();
    //     cmd.args(["pattern", "path"])
    //         .assert()
    //         .failure()
    //         .code(1)
    //         .stderr(predicate::str::contains("Error message"));
    //     cmd.assert().success();
    //     // let output = cmd.unwrap();
    // }
}
