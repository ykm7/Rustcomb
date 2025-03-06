use std::{
    fs::{self, DirEntry},
    io::Error,
    path::Path,
};

use clap::Parser;
#[warn(unused_imports)]
use regex::Regex;

// /// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
struct Cli {
    /// The pattern to look for
    pattern: String,
    /// The path to the file to read
    path: std::path::PathBuf,
}

fn visit_dirs(dir: &Path, cd: &dyn Fn(&DirEntry)) -> Result<(), Error> {
    // if dir.is_dir() {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            visit_dirs(&path, cd)?;
        } else {
            cd(&entry);
        }
    }

    Ok(())
    // }
}

fn main() {
    let args2 = Cli::parse_from(wild::args_os());

    println!("{}", args2.pattern);
    println!("{:?}", args2.path);

    let closure = |entry: &DirEntry| {
        let name = entry.file_name();
        println!("{:?}", name);
    };

    let _ = visit_dirs(&args2.path, &closure);

    // let expression = Regex::new(&args2.pattern);
}
