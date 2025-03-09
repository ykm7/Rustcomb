use clap::Parser;
use core::fmt;
use rayon::prelude::*;
use regex::Regex;
use std::error::Error;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::thread;
use threadpool::ThreadPool;
use walkdir::WalkDir;

#[derive(Parser, Clone)]
#[clap(name = "Rustcomb")]
pub struct Cli {
    /// The pattern to look for
    pub path_pattern: String,
    /// The path to the file to read
    pub path: std::path::PathBuf,

    pub file_pattern: String,
}

impl std::fmt::Debug for Cli {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Path pattern: {:?}, Path: {:?}, File pattern: {}",
            self.path_pattern, self.path, self.file_pattern
        )
    }
}

#[inline]
pub fn single_thread_read_files(args: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let file_pattern_re: Regex = match clean_up_regex(&args.path_pattern) {
        Err(err) => {
            panic!(
                "Unable to accept pattern as valid regex: {} with err: {}",
                args.path_pattern, err
            );
        }
        Ok(re) => re,
    };

    let iterator = find_files(&args.path, &file_pattern_re);
    use_single_thread(iterator, &args, false)?;
    Ok(())
}

#[inline]
pub fn rayon_read_files(args: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let file_pattern_re: Regex = match clean_up_regex(&args.path_pattern) {
        Err(err) => {
            panic!(
                "Unable to accept pattern as valid regex: {} with err: {}",
                args.path_pattern, err
            );
        }
        Ok(re) => re,
    };

    let rayon_iterator = rayon_find_files(&args.path, &file_pattern_re);

    match use_rayon(rayon_iterator, &args, false) {
        Err(err) => {
            eprintln!("Error on 'rayon_read_files': {:?}", err)
        }
        Ok(r) => r,
    };

    Ok(())
}

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

fn clean_up_regex(pattern: &str) -> Result<regex::Regex, regex::Error> {
    let escaped = regex::escape(pattern);
    let replaced = escaped.replace("\\*", ".*").to_string();
    Regex::new(replaced.as_str())
}

fn use_single_thread<I>(
    iterator: I,
    args: &Cli,
    debug: bool,
) -> Result<(), Box<dyn std::error::Error>>
where
    I: Iterator<Item = Result<FileInfo, Box<dyn Error>>>,
{
    let string_pattern_re = clean_up_regex(&args.file_pattern)?;
    let results: Vec<(FileInfo, Vec<String>)> = iterator
        .filter_map(|item| match item {
            Ok(file) => Some(file),
            Err(err) => {
                eprintln!("Error parsing item: {:?}", err);
                None
            }
        })
        .filter_map(
            |file| match find_entry_within_file(&file, &string_pattern_re) {
                Err(err) => {
                    eprintln!("Error while searching file {}", err);
                    None
                }
                Ok(found) => Some((file, found)),
            },
        )
        .collect();

    if debug {
        let found_matches_count = results.len();
        println!("Found {} matches.", found_matches_count);
    }
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
            thread::spawn(move || match find_entry_within_file(&file, &re) {
                Err(err) => {
                    eprintln!("Error while searching file {}", err);
                    Vec::new()
                }
                Ok(found) => found,
            });

        handles.push((interal_file, handle));
    }

    let results = handles.into_iter().map(|f| (f.0, f.1.join().unwrap()));
    if debug {
        let found_matches_count = results.len();
        println!("Found {} matches.", found_matches_count);
    }
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

        pool.execute(move || match find_entry_within_file(&file, &re) {
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
    if debug {
        let found_matches_count = results.len();
        println!("Found {} matches.", found_matches_count);
    }
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

fn use_rayon<I>(iterator: I, args: &Cli, debug: bool) -> Result<(), Box<dyn Error + Send + Sync>>
where
    I: ParallelIterator<Item = Result<FileInfo, Box<dyn Error + Send + Sync>>>,
{
    let string_pattern_re = Arc::new(clean_up_regex(&args.file_pattern)?);

    let results: Vec<_> = iterator
        .filter_map(|item| match item {
            Ok(file) => Some(file),
            Err(err) => {
                eprintln!("Error parsing item: {:?}", err);
                None
            }
        })
        .map(|file| {
            let re: Arc<Regex> = Arc::clone(&string_pattern_re);
            let internal_file = file.clone();

            match find_entry_within_file(&file, &re) {
                Err(err) => {
                    eprintln!("Error while searching file {}", err);
                    (internal_file, Vec::new())
                }
                Ok(found) => (internal_file, found),
            }
        })
        .collect();

    if debug {
        let found_matches_count = results.len();
        println!("Found {} matches.", found_matches_count);
    }
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

fn find_files(dir: &Path, re: &Regex) -> impl Iterator<Item = Result<FileInfo, Box<dyn Error>>> {
    // let re_clone = re.clone();
    let iterator = WalkDir::new(dir).into_iter().filter_map(|entry| {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => return Some(Err(err.into())),
        };

        if !entry.file_type().is_file() {
            return None;
        }

        let path = entry.path();
        let filename = path.file_name().and_then(|os_str| os_str.to_str());

        match filename {
            Some(filename) => {
                if re.is_match(filename) {
                    Some(Ok(FileInfo {
                        path: path.to_path_buf(),
                        filename: filename.to_string(),
                    }))
                } else {
                    None
                }
            }
            None => None,
        }
    });
    iterator
}

fn rayon_find_files(
    dir: &Path,
    re: &Regex,
) -> impl ParallelIterator<Item = Result<FileInfo, Box<dyn Error + Send + Sync>>> {
    let iterator = WalkDir::new(dir)
        .into_iter()
        .par_bridge()
        .filter_map(|entry| {
            let entry = match entry {
                Ok(e) => e,
                Err(err) => return Some(Err(err.into())),
            };

            if !entry.file_type().is_file() {
                return None;
            }

            let path = entry.path();
            let filename = path.file_name().and_then(|os_str| os_str.to_str());

            match filename {
                Some(filename) => {
                    if re.is_match(filename) {
                        Some(Ok(FileInfo {
                            path: path.to_path_buf(),
                            filename: filename.to_string(),
                        }))
                    } else {
                        None
                    }
                }
                None => None,
            }
        });

    iterator
}

fn find_entry_within_file(
    f: &FileInfo,
    re: &Regex,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let file = File::open(&f.path)?;
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
