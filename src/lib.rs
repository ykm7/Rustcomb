use clap::Parser;
use core::fmt;
use rayon::prelude::*;
use regex::Regex;
use std::error;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::thread;
use threadpool::ThreadPool;
use walkdir::WalkDir;

#[derive(Debug)]
pub enum MyErrors {
    Regex(regex::Error),
    WalkDir(walkdir::Error),
    FileIO(io::Error),
}

impl fmt::Display for MyErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            MyErrors::Regex(ref e) => write!(f, "regex error: ({}", e),
            MyErrors::WalkDir(ref e) => write!(f, "WalkDir error: ({}", e),
            MyErrors::FileIO(ref e) => write!(f, "File IO eror: ({}", e),
        }
    }
}

impl error::Error for MyErrors {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            MyErrors::Regex(ref e) => Some(e),
            MyErrors::WalkDir(ref e) => Some(e),
            MyErrors::FileIO(ref e) => Some(e),
        }
    }
}

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
pub fn single_thread_read_files(args: Cli) -> Result<(), MyErrors> {
    let file_pattern_re = clean_up_regex(&args.path_pattern)?;
    let iterator = find_files(&args.path, &file_pattern_re);
    use_single_thread(iterator, &file_pattern_re, false)?;
    Ok(())
}

#[inline]
pub fn rayon_read_files(args: Cli) -> Result<(), MyErrors> {
    let file_pattern_re = clean_up_regex(&args.path_pattern)?;
    let rayon_iterator = rayon_find_files(&args.path, &file_pattern_re);
    use_rayon(rayon_iterator, &file_pattern_re, false)?;

    Ok(())
}

#[inline]
pub fn thread_per_file_read_files(args: Cli) -> Result<(), MyErrors> {
    let file_pattern_re = clean_up_regex(&args.path_pattern)?;
    let iterator = find_files(&args.path, &file_pattern_re);
    use_thread_per_file(iterator, &file_pattern_re, false)?;

    Ok(())
}

#[inline]
pub fn threadpool_read_files(args: Cli, number_of_workers: usize) -> Result<(), MyErrors> {
    let file_pattern_re = clean_up_regex(&args.path_pattern)?;
    let iterator = find_files(&args.path, &file_pattern_re);
    use_thread_pool(iterator, &file_pattern_re, false, number_of_workers)?;

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

fn clean_up_regex(pattern: &str) -> Result<regex::Regex, MyErrors> {
    let escaped = regex::escape(pattern);
    let replaced = escaped.replace("\\*", ".*").to_string();
    Regex::new(replaced.as_str()).map_err(MyErrors::Regex)
}

fn use_single_thread<I>(iterator: I, re: &Regex, debug: bool) -> Result<(), MyErrors>
where
    I: Iterator<Item = Result<FileInfo, MyErrors>>,
{
    let results: Vec<(FileInfo, Vec<String>)> = iterator
        .filter_map(|item| match item {
            Ok(file) => Some(file),
            Err(err) => {
                eprintln!("Error parsing item: {}", err);
                None
            }
        })
        .filter_map(|file| match find_entry_within_file(&file, re) {
            Err(err) => {
                eprintln!("Error while searching file {}", err);
                None
            }
            Ok(found) => Some((file, found)),
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

/**
 * This is the initial implementation using thread::spawn
 */
fn use_thread_per_file<I>(iterator: I, re: &Regex, debug: bool) -> Result<(), MyErrors>
where
    I: Iterator<Item = Result<FileInfo, MyErrors>>,
{
    let matched_paths = iterator.filter_map(|r| r.ok()).collect::<Vec<FileInfo>>();

    let mut handles = Vec::new();
    let re = Arc::new(re.to_owned());
    for file in matched_paths {
        let re: Arc<Regex> = Arc::clone(&re);
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

fn use_thread_pool<I>(
    iterator: I,
    re: &Regex,
    debug: bool,
    number_of_workers: usize,
) -> Result<(), MyErrors>
where
    I: Iterator<Item = Result<FileInfo, MyErrors>>,
{
    let matched_paths = iterator.filter_map(|r| r.ok()).collect::<Vec<FileInfo>>();

    let number_of_jobs = matched_paths.len();
    let pool = ThreadPool::new(number_of_workers);
    let re = Arc::new(re.to_owned());

    let (tx, rx) = channel();
    for file in matched_paths {
        let tx = tx.clone();
        let re: Arc<Regex> = Arc::clone(&re);
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

fn use_rayon<I>(iterator: I, re: &Regex, debug: bool) -> Result<(), MyErrors>
where
    I: ParallelIterator<Item = Result<FileInfo, MyErrors>>,
{
    let re = Arc::new(re.to_owned());
    let results: Vec<_> = iterator
        .filter_map(|item| match item {
            Ok(file) => Some(file),
            Err(err) => {
                eprintln!("Error parsing item: {:?}", err);
                None
            }
        })
        .map(|file| {
            let re: Arc<Regex> = Arc::clone(&re);
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

fn find_files(dir: &Path, re: &Regex) -> impl Iterator<Item = Result<FileInfo, MyErrors>> {
    // let re_clone = re.clone();
    let iterator = WalkDir::new(dir).into_iter().filter_map(|entry| {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => return Some(Err(MyErrors::WalkDir(err))),
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
) -> impl ParallelIterator<Item = Result<FileInfo, MyErrors>> {
    let iterator = WalkDir::new(dir)
        .into_iter()
        .par_bridge()
        .filter_map(|entry| match entry {
            Ok(entry) if entry.file_type().is_file() => Some(entry),
            Ok(_) => None,
            Err(err) => {
                eprintln!("Error reading entry: {}", err);
                None
            }
        })
        .filter_map(|entry| {
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

fn find_entry_within_file(f: &FileInfo, re: &Regex) -> Result<Vec<String>, MyErrors> {
    let file = File::open(&f.path).map_err(MyErrors::FileIO)?;
    let reader = BufReader::new(file);

    let mut found_lines = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line.map_err(MyErrors::FileIO)?;
        if re.is_match(&line) {
            found_lines.push(format!("Line {} - {}", idx, line));
        }
    }

    Ok(found_lines)
}
