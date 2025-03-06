use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use regex::Regex;
// #[warn(unused_imports)]
// use regex::Regex;

struct FileInfo {
    path: PathBuf,
    filename: String,
}

// /// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
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
            let filename = entry
                .file_name()
                .into_string()
                .map_err(|err| format!("Failed to convert OsString to String: '{:?}'", err))?;

            if re.is_match(&filename) {
                matched_paths.push(FileInfo { path, filename });
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

fn setup<I, T>(args: I) -> Result<(), Box<dyn Error>>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let args = Cli::parse_from(args);

    let mut matched_paths: Vec<FileInfo> = Vec::new();

    let file_pattern_re: Regex = match Regex::new(&args.path_pattern) {
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
    println!("Time taken for total (identifying files): {:?}", duration);

    let num_found = matched_paths.len();
    println!("Found {} files to examine", num_found);
    if num_found == 0 {
        return Ok(());
    }

    let string_pattern_re = Regex::new(&args.file_pattern)?;
    for file in matched_paths {
        println!("Path: {:?}", file.path);
        println!("Filename: {:?}", file.filename);

        let found_matches = find_entry_within_file(file, &string_pattern_re)?;
        let found_matches_count = found_matches.len();
        if found_matches_count != 0 {
            println!("Found {} matches.", found_matches_count);
            for m in found_matches {
                println!("{}", m);
            }
        }
    }

    Ok(())
}

fn main() {
    let _ = setup(wild::args_os());
}

#[cfg(test)]
mod tests {
    use super::*;
    // use assert_cmd::Command;
    // use predicates::prelude::*;

    #[test]
    fn test_setup() {
        assert!(setup(["Rustcomb", "PATTERN", "."]).is_ok());
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
