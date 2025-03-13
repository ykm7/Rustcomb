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

    // let mut handle = BufWriter::new(io::stdout());
    // let mut output = String::new();

    let start = Instant::now();
    rustcomb::single_thread_read_files(Arc::clone(&cli), print)?;
    let single_thread = start.elapsed();
    println!(
        "{}",
        Colour::Green.paint(format!(
            "Time taken for identifying files (single_thread_read_files): {:?}",
            single_thread
        ))
    );

    let start = Instant::now();
    rustcomb::thread_per_file_read_files(Arc::clone(&cli), print)?;
    let thread_per_file_elapsed = start.elapsed();
    println!(
        "{}",
        Colour::Green.paint(format!(
            "Time taken for identifying files (use_thread_per_file): {:?}",
            thread_per_file_elapsed
        ))
    );

    let start = Instant::now();
    rustcomb::threadpool_read_files(Arc::clone(&cli), print, 1)?;
    let threadpool_single_elapsed = start.elapsed();
    println!(
        "{}",
        Colour::Green.paint(format!(
            "Time taken for identifying files (use_thread_pool - 1 thread): {:?}",
            threadpool_single_elapsed
        ))
    );

    let cpus = num_cpus::get();
    let start = Instant::now();
    rustcomb::threadpool_read_files(Arc::clone(&cli), print, cpus)?;
    let threadpool_multiple_elapsed = start.elapsed();
    println!(
        "{}",
        Colour::Green.paint(format!(
            "Time taken for identifying files (use_thread_pool - {} thread): {:?}",
            cpus, threadpool_multiple_elapsed
        ))
    );

    let start = Instant::now();
    rustcomb::rayon_read_files(Arc::clone(&cli), print)?;
    let rayon_elapsed = start.elapsed();
    println!(
        "{}",
        Colour::Green.paint(format!(
            "Time taken for identifying files (rayon_read_files): {:?}",
            rayon_elapsed
        ))
    );

    let mut handle = BufWriter::new(io::stdout());
    let mut output = String::new();

    output.push('\n');
    output.push_str(&format!(
        "{}\n",
        Colour::Green.paint(format!(
            "Time taken for identifying files (single_thread_read_files): {:?}",
            single_thread
        ))
    ));
    output.push_str(&format!(
        "{}\n",
        Colour::Green.paint(format!(
            "Time taken for identifying files (use_thread_per_file): {:?}",
            thread_per_file_elapsed
        ))
    ));
    output.push_str(&format!(
        "{}\n",
        Colour::Green.paint(format!(
            "Time taken for identifying files (use_thread_pool - 1 thread): {:?}",
            threadpool_single_elapsed
        ))
    ));
    output.push_str(&format!(
        "{}\n",
        Colour::Green.paint(format!(
            "Time taken for identifying files (use_thread_pool - {} thread): {:?}",
            cpus, threadpool_multiple_elapsed
        ))
    ));
    output.push_str(&format!(
        "{}\n",
        Colour::Green.paint(format!(
            "Time taken for identifying files (rayon_read_files): {:?}",
            rayon_elapsed
        ))
    ));

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
