// mod lib;
// pub use main::Cli;
// pub use main::{single_thread_read_files,rayon_read_files};

// use clap::Parser;
// use core::fmt;
// use rayon::prelude::*;
// use regex::Regex;
use clap::Parser;
use std::{error::Error, time::Instant};
// use std::fs::File;
// use std::io::BufRead;
// use std::io::BufReader;
// use std::path::Path;
// use std::path::PathBuf;
// use std::sync::Arc;
// use std::sync::mpsc::channel;
// use std::thread;
// use std::time::Instant;
// use threadpool::ThreadPool;
// use walkdir::WalkDir;

// pub struct Cli {
//     /// The pattern to look for
//     pub path_pattern: String,
//     /// The path to the file to read
//     pub path: std::path::PathBuf,

//     pub file_pattern: String,
// }

// impl std::fmt::Debug for Cli {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "Path pattern: {:?}, Path: {:?}, File pattern: {}",
//             self.path_pattern, self.path, self.file_pattern
//         )
//     }
// }

// #[inline]
// pub fn single_thread_read_files(args: Cli) -> Result<(), Box<dyn std::error::Error>> {
//     let file_pattern_re: Regex = match clean_up_regex(&args.path_pattern) {
//         Err(err) => {
//             panic!(
//                 "Unable to accept pattern as valid regex: {} with err: {}",
//                 args.path_pattern, err
//             );
//         }
//         Ok(re) => re,
//     };

//     let iterator = find_files(&args.path, &file_pattern_re);
//     use_single_thread(iterator, &args, false)?;
//     Ok(())
// }

// #[inline]
// pub fn rayon_read_files(args: Cli) -> Result<(), Box<dyn std::error::Error>> {
//     let file_pattern_re: Regex = match clean_up_regex(&args.path_pattern) {
//         Err(err) => {
//             panic!(
//                 "Unable to accept pattern as valid regex: {} with err: {}",
//                 args.path_pattern, err
//             );
//         }
//         Ok(re) => re,
//     };

//     let rayon_iterator = rayon_find_files(&args.path, &file_pattern_re);

//     match use_rayon(rayon_iterator, &args, false) {
//         Err(err) => {
//             eprintln!("Error on 'rayon_read_files': {:?}", err)
//         }
//         Ok(r) => r,
//     };

//     Ok(())
// }

// /// Search for a pattern in a file and display the lines that contain it.

fn setup(args: rustcomb::Cli) -> Result<(), Box<dyn Error>> {
    println!("Args - path_pattern: {:?}", args.path_pattern);
    println!("Args - path: {:?}", args.path);
    println!("Args - file_pattern: {:?}", args.file_pattern);

    // let mut matched_paths: Vec<FileInfo> = Vec::new();

    let args_clone = args.clone();
    let start = Instant::now();
    rustcomb::single_thread_read_files(args_clone)?;
    println!(
        "Time taken for identifying files (single_thread_read_files): {:?}",
        start.elapsed()
    );

    let args_clone = args.clone();
    let start = Instant::now();
    rustcomb::rayon_read_files(args_clone)?;
    println!(
        "Time taken for identifying files (rayon_read_files): {:?}",
        start.elapsed()
    );
    // let start = Instant::now();
    // let rayon_iterator = rayon_find_files(&args.path, &file_pattern_re);
    // let duration = start.elapsed();
    // println!("Time taken for identifying files (rayon): {:?}", duration);

    // let num_found = matched_paths.len();
    // println!("Found {} files to examine", num_found);
    // if num_found == 0 {
    //     return Ok(());
    // }

    // let matched_paths: Vec<_> = iterator.collect::<Result<Vec<_>, _>>()?;

    // let start_single_thread = Instant::now();
    // use_single_thread(iterator, args.clone(), false)?;
    // println!(
    //     "Time taken to search through files using a single thread: {:?} - total: {:?}",
    //     start_single_thread.elapsed(),
    //     start.elapsed()
    // );

    // let matched_paths = iterator.collect::<Vec<Result<_, _>>>();

    // let start_thread_per_file = Instant::now();
    // use_thread_per_file(matched_paths, args.clone(), false)?;
    // println!(
    //     "Time taken to search through files using a thread per each file: {:?} - total: {:?}",
    //     start_thread_per_file.elapsed(),
    //     start.elapsed()
    // );

    // let start_thread_pool_1 = Instant::now();
    // use_thread_pool(matched_paths.clone(), args.clone(), false, 1)?;
    // println!(
    //     "Time taken to search through files using a thread pool (thread of 1): {:?} - total: {:?}",
    //     start_thread_pool_1.elapsed(),
    //     start.elapsed()
    // );

    // let num_cpus = num_cpus::get();
    // let number_of_workers = num_cpus;
    // let start_thread_pool_num_cpus = Instant::now();
    // use_thread_pool(
    //     matched_paths.clone(),
    //     args.clone(),
    //     false,
    //     number_of_workers,
    // )?;
    // println!(
    //     "Time taken to search through files using a thread pool (thread of {}): {:?} - total: {:?}",
    //     num_cpus,
    //     start_thread_pool_num_cpus.elapsed(),
    //     start.elapsed()
    // );

    // let start_rayon = Instant::now();
    // use_rayon(rayon_iterator, args.clone(), false);
    // println!(
    //     "Time taken to search through files using Rayon: {:?} - total: {:?}",
    //     start_rayon.elapsed(),
    //     start.elapsed()
    // );

    Ok(())
}

fn main() {
    let args = rustcomb::Cli::parse_from(wild::args_os());
    if let Err(e) = setup(args) {
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

        let args = rustcomb::Cli::parse_from(args);
        // Use setup_with_args instead of setup to pass custom arguments
        assert!(setup(args).is_ok());
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
