use std::error::Error;
use std::fs;
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
    pattern: String,
    /// The path to the file to read
    path: std::path::PathBuf,
}

fn visit_dirs(
    dir: &Path,
    re: &Regex,
    matched_paths: &mut Vec<FileInfo>,
) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path: PathBuf = entry.path();

        if path.is_dir() {
            visit_dirs(&path, re, matched_paths)?;
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

fn setup<I, T>(args: I) -> Result<(), Box<dyn Error>>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let args = Cli::parse_from(args);

    let mut matched_paths: Vec<FileInfo> = Vec::new();

    let re: Regex = match Regex::new(&args.pattern) {
        Err(err) => {
            panic!(
                "Unable to accept pattern as valid regex: {} with err: {}",
                args.pattern, err
            );
        }
        Ok(re) => re,
    };

    let start = Instant::now();
    if let Err(err) = visit_dirs(&args.path, &re, &mut matched_paths) {
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

    for file in matched_paths {
        println!("Path: {:?}", file.path);
        println!("Filename: {:?}", file.filename);
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
