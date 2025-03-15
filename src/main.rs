use ansi_term::Colour;
use clap::Parser;
use std::{
    error::Error,
    io::{self, BufWriter, Write},
    sync::Arc,
    time::Instant,
};
use wild::args_os;

fn setup(args: rustcomb::Cli, print: bool) -> Result<(), Box<dyn Error>> {
    println!("Args: {:?}", args);
    let cli = Arc::new(args);

    let start = Instant::now();
    rustcomb::single_thread_read_files(Arc::clone(&cli), print)?;
    let single_thread = start.elapsed();
    let single_thread_print = format!(
        "{}",
        Colour::Green.paint(format!(
            "Time taken for identifying files (single_thread_read_files): {:?}",
            single_thread
        ))
    );
    println!("{single_thread_print}");

    let start = Instant::now();
    rustcomb::thread_per_file_read_files(Arc::clone(&cli), print)?;
    let thread_per_file_elapsed = start.elapsed();
    let thread_per_file_elapsed_print = format!(
        "{}",
        Colour::Green.paint(format!(
            "Time taken for identifying files (use_thread_per_file): {:?}",
            thread_per_file_elapsed
        ))
    );
    println!("{thread_per_file_elapsed_print}");

    let start = Instant::now();
    rustcomb::threadpool_read_files(Arc::clone(&cli), print, 1)?;
    let threadpool_single_elapsed = start.elapsed();
    let threadpool_single_elapsed_print = format!(
        "{}",
        Colour::Green.paint(format!(
            "Time taken for identifying files (use_thread_pool - 1 thread): {:?}",
            threadpool_single_elapsed
        ))
    );
    println!("{threadpool_single_elapsed_print}");

    let cpus = num_cpus::get();
    let start = Instant::now();
    rustcomb::threadpool_read_files(Arc::clone(&cli), print, cpus)?;
    let threadpool_multiple_elapsed = start.elapsed();
    let threadpool_multiple_elapsed_print = format!(
        "{}",
        Colour::Green.paint(format!(
            "Time taken for identifying files (use_thread_pool - {} thread): {:?}",
            cpus, threadpool_multiple_elapsed
        ))
    );
    println!("{threadpool_multiple_elapsed_print}");

    let start = Instant::now();
    rustcomb::rayon_read_files(Arc::clone(&cli), print)?;
    let rayon_elapsed = start.elapsed();
    let rayon_elapsed_print = format!(
        "{}",
        Colour::Green.paint(format!(
            "Time taken for identifying files (rayon_read_files): {:?}",
            rayon_elapsed
        ))
    );
    println!("{rayon_elapsed_print}");

    let mut handle = BufWriter::new(io::stdout());
    let mut output = String::new();

    output.push('\n');
    output.push_str(&single_thread_print.to_string());
    output.push('\n');
    output.push_str(&thread_per_file_elapsed_print.to_string());
    output.push('\n');
    output.push_str(&threadpool_single_elapsed_print.to_string());
    output.push('\n');
    output.push_str(&threadpool_multiple_elapsed_print.to_string());
    output.push('\n');
    output.push_str(&rayon_elapsed_print.to_string());
    output.push('\n');

    handle.write_all(output.as_bytes())?;

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
        let args = vec![
            "Rustcomb",
            "*.txt",
            ".\\test_files",
            "metus mus. Elit convallis",
        ];
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
