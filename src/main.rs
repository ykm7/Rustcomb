use clap::Parser;
use core::fmt;
use regex::Regex;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Instant;
use threadpool::ThreadPool;
use rayon::prelude::*;

#[derive(Clone)]
struct FileInfo {
    path: PathBuf,
    filename: String,
}

impl fmt::Display for FileInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Path: {:?}, Filename: {}", self.path, self.filename)
    }
}
// /// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser, Clone)]
#[clap(name = "Rustcomb")]
struct Cli {
    /// The pattern to look for
    path_pattern: String,
    /// The path to the file to read
    path: std::path::PathBuf,

    file_pattern: String,
}

fn find_files(
    dir: &Path,
    re: &Regex,
    matched_paths: &mut Vec<FileInfo>,
) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path: PathBuf = entry.path();

        if path.is_dir() {
            find_files(&path, re, matched_paths)?;
        } else {
            let filename = path
                .file_name()
                .and_then(|os_str| os_str.to_str())
                .ok_or_else(|| format!("Invalid filename: {:?}", path))?;
            // .map_err(|err| format!("Failed to convert OsString to String: '{:?}'", err))?;

            if re.is_match(filename) {
                matched_paths.push(FileInfo {
                    path: path.clone(),
                    filename: filename.to_string(),
                });
            }
        }
    }

    Ok(())
}

fn find_entry_within_file(
    f: FileInfo,
    re: &Regex,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let file = File::open(f.path)?;
    let reader = BufReader::new(file);

    let mut found_lines = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line?;
        if re.is_match(&line) {
            found_lines.push(format!("Line {} - {}", idx, line));
        }
    }

    Ok(found_lines)
}

fn clean_up_regex(pattern: &str) -> Result<regex::Regex, regex::Error> {
    let escaped = regex::escape(pattern);
    let replaced = escaped.replace("\\*", ".*").to_string();
    Regex::new(replaced.as_str())
}

fn setup(args: Cli) -> Result<(), Box<dyn Error>> {
    println!("Args - path_pattern: {:?}", args.path_pattern);
    println!("Args - path: {:?}", args.path);
    println!("Args - file_pattern: {:?}", args.file_pattern);

    let mut matched_paths: Vec<FileInfo> = Vec::new();

    let file_pattern_re: Regex = match clean_up_regex(&args.path_pattern) {
        Err(err) => {
            panic!(
                "Unable to accept pattern as valid regex: {} with err: {}",
                args.path_pattern, err
            );
        }
        Ok(re) => re,
    };

    let start = Instant::now();
    if let Err(err) = find_files(&args.path, &file_pattern_re, &mut matched_paths) {
        println!("{:?}", err);
        return Err(err);
    }
    let duration = start.elapsed();
    println!("Time taken for identifying files: {:?}", duration);

    let num_found = matched_paths.len();
    println!("Found {} files to examine", num_found);
    if num_found == 0 {
        return Ok(());
    }

    let start = Instant::now();
    use_thread_per_file(matched_paths.clone(), args.clone(), false)?;
    let duration = start.elapsed();
    println!(
        "Time taken to search through files using a thread per each file: {:?}",
        duration
    );

    let start = Instant::now();
    use_thread_pool(matched_paths.clone(), args.clone(), false, 1)?;
    let duration = start.elapsed();
    println!(
        "Time taken to search through files using a thread pool (thread of 1): {:?}",
        duration
    );

    let num_cpus = num_cpus::get();
    let number_of_workers = num_cpus;
    let start = Instant::now();
    use_thread_pool(
        matched_paths.clone(),
        args.clone(),
        false,
        number_of_workers,
    )?;
    let duration = start.elapsed();
    println!(
        "Time taken to search through files using a thread pool (thread of {}): {:?}",
        num_cpus, duration,
    );

    let start = Instant::now();
    use_rayon(matched_paths.clone(), args.clone(), false)?;
    let duration = start.elapsed();
    println!(
        "Time taken to search through files using Rayon: {:?}",
        duration
    );

    Ok(())
}

/**
 * This is the initial implementation using thread::spawn
 */
fn use_thread_per_file(
    matched_paths: Vec<FileInfo>,
    args: Cli,
    debug: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut handles = Vec::new();
    let string_pattern_re = Arc::new(clean_up_regex(&args.file_pattern)?);
    for file in matched_paths {
        let re: Arc<Regex> = Arc::clone(&string_pattern_re);
        let interal_file = file.clone();
        let handle: thread::JoinHandle<Vec<String>> =
            thread::spawn(move || match find_entry_within_file(interal_file, &re) {
                Err(err) => {
                    eprintln!("Error while searching file {}", err);
                    Vec::new()
                }
                Ok(found) => found,
            });

        handles.push((file, handle));
    }

    let results = handles.into_iter().map(|f| (f.0, f.1.join().unwrap()));
    let found_matches_count = results.len();

    println!("Found {} matches.", found_matches_count);
    for (f, r) in results {
        if !r.is_empty() && debug {
            println!("Filename found with matches: {}", f);
            for m in r {
                println!("{}", m);
            }
        }
    }

    Ok(())
}

fn use_thread_pool(
    matched_paths: Vec<FileInfo>,
    args: Cli,
    debug: bool,
    number_of_workers: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Using {} number of workers", number_of_workers);
    let number_of_jobs = matched_paths.len();
    let pool = ThreadPool::new(number_of_workers);
    let string_pattern_re = Arc::new(clean_up_regex(&args.file_pattern)?);

    let (tx, rx) = channel();
    for file in matched_paths {
        let tx = tx.clone();
        let re: Arc<Regex> = Arc::clone(&string_pattern_re);
        let internal_file = file.clone();

        pool.execute(move || match find_entry_within_file(file.clone(), &re) {
            Err(err) => {
                eprintln!("Error while searching file {}", err);
                tx.send((internal_file, Vec::new()))
                    .expect("Critical error when handling error on file internal search");
            }
            Ok(found) => {
                tx.send((internal_file, found))
                    .expect("Critical error while handling successful file internal searc");
            }
        });
    }

    let results: Vec<_> = rx.iter().take(number_of_jobs).collect();
    let found_matches_count = results.len();
    println!("Found {} matches.", found_matches_count);
    for (f, r) in results {
        if !r.is_empty() && debug {
            println!("Filename found with matches: {}", f);
            for m in r {
                println!("{}", m);
            }
        }
    }

    Ok(())
}

fn use_rayon(
    matched_paths: Vec<FileInfo>,
    args: Cli,
    debug: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let string_pattern_re = Arc::new(clean_up_regex(&args.file_pattern)?);

    let results: Vec<_> = matched_paths
        .par_iter()
        .map(|file| {
            let re: Arc<Regex> = Arc::clone(&string_pattern_re);
            let internal_file = file.clone();

            match find_entry_within_file(file.clone(), &re) {
                Err(err) => {
                    eprintln!("Error while searching file {}", err);
                    (internal_file, Vec::new())
                }
                Ok(found) => (internal_file, found),
            }
        })
        .collect();

    let found_matches_count = results.len();
    println!("Found {} matches.", found_matches_count);
    for (f, r) in results {
        if !r.is_empty() && debug {
            println!("Filename found with matches: {}", f);
            for m in r {
                println!("{}", m);
            }
        }
    }

    Ok(())
}

fn main() {
    let args = Cli::parse_from(wild::args_os());
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

        let args = Cli::parse_from(args);
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
