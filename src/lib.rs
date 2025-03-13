use ansi_term::Colour;
use clap::Parser;
use core::fmt;
use lazy_static::lazy_static;
use rayon::prelude::*;
use regex::Regex;
use std::borrow::Cow;
use std::error;
use std::error::Error;
use std::fmt::Display;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::{self, Write};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::PoisonError;
use std::sync::mpsc::channel;
use std::thread;
use threadpool::ThreadPool;
use walkdir::WalkDir;

#[derive(Debug)]
pub enum MyErrors {
    Regex(regex::Error),
    WalkDir(walkdir::Error),
    FileIO(io::Error),
    LockError(String),
}

lazy_static! {
    static ref STAR_PATTERN: Regex = Regex::new("\\*").unwrap();
}

impl fmt::Display for MyErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            MyErrors::Regex(ref e) => write!(f, "regex error: ({})", e),
            MyErrors::WalkDir(ref e) => write!(f, "WalkDir error: ({})", e),
            MyErrors::FileIO(ref e) => write!(f, "File IO error: ({})", e),
            MyErrors::LockError(ref e) => write!(f, "Lock error ({})", e),
        }
    }
}

impl error::Error for MyErrors {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            MyErrors::Regex(ref e) => Some(e),
            MyErrors::WalkDir(ref e) => Some(e),
            MyErrors::FileIO(ref e) => Some(e),
            MyErrors::LockError(_) => None,
        }
    }
}

impl<T> From<PoisonError<T>> for MyErrors
where
    T: Display, // PoisonError<T> implements Display regardless of T
{
    fn from(err: PoisonError<T>) -> Self {
        MyErrors::LockError(format!("Mutex/RwLock poisoned: {}", err))
    }
}

#[derive(Parser, Clone, Debug)]
#[clap(name = "Rustcomb")]
pub struct Cli {
    // The file name pattern to look for
    pub path_pattern: String,
    // The path to the file to read
    pub path: std::path::PathBuf,
    // The file pattern to look for
    pub file_pattern: String,
}

impl std::fmt::Display for Cli {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Path pattern: {}, Path: {:?}, File pattern: {}",
            self.path_pattern, self.path, self.file_pattern
        )
    }
}

#[inline]
pub fn single_thread_read_files(args: Arc<Cli>, print: bool) -> Result<(), MyErrors> {
    let path_pattern = clean_up_regex(&args.path_pattern)?;
    let iterator = find_files(&args.path, &path_pattern);
    let file_pattern_re = clean_up_regex(&args.file_pattern)?;
    use_single_thread(iterator, &file_pattern_re, print)?;
    Ok(())
}

#[inline]
pub fn rayon_read_files(args: Arc<Cli>, print: bool) -> Result<(), MyErrors> {
    let path_pattern = clean_up_regex(&args.path_pattern)?;
    let rayon_iterator = rayon_find_files(&args.path, &path_pattern);
    let file_pattern_re = clean_up_regex(&args.file_pattern)?;
    use_rayon(rayon_iterator, &file_pattern_re, print)?;

    Ok(())
}

#[inline]
pub fn thread_per_file_read_files(args: Arc<Cli>, print: bool) -> Result<(), MyErrors> {
    let path_pattern = clean_up_regex(&args.path_pattern)?;
    let iterator = find_files(&args.path, &path_pattern);
    let file_pattern_re = clean_up_regex(&args.file_pattern)?;
    use_thread_per_file(iterator, &file_pattern_re, print)?;

    Ok(())
}

#[inline]
pub fn threadpool_read_files(
    args: Arc<Cli>,
    print: bool,
    number_of_workers: usize,
) -> Result<(), MyErrors> {
    let path_pattern = clean_up_regex(&args.path_pattern)?;
    let iterator = find_files(&args.path, &path_pattern);
    let file_pattern_re = clean_up_regex(&args.file_pattern)?;
    use_thread_pool(iterator, &file_pattern_re, print, number_of_workers)?;

    Ok(())
}

struct FileInfo {
    path: PathBuf,
    filename: String,
}

impl FileInfo {
    fn get_identifier(&self) -> String {
        format!("{}", &self)
    }
}

impl fmt::Display for FileInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = Colour::Green.paint(format!("{} {:?}", self.filename, self.path));
        // let s = Style::new().bold().paint(format!("Path: {:?}, Filename: {}", self.path, filename));
        write!(f, "{}", s)
    }
}

/**
 * Use a single initialised re pattern to save it being created on each call (STAR_PATTERN)
 *
 * re.Replace() returns a COW (Copy on Write) type.
 * So a new string is only created when its modified.
 * Can be used whether using the Borrowed or the Owned
 */
fn clean_up_regex(pattern: &str) -> Result<regex::Regex, MyErrors> {
    let escaped = regex::escape(pattern);
    let cleaned: std::borrow::Cow<'_, str> = STAR_PATTERN.replace(&escaped, ".*");
    Regex::new(&cleaned).map_err(MyErrors::Regex)
}

fn information_out(results: &Vec<(String, Vec<String>)>) -> Result<(), MyErrors> {
    let found_matches_count = results.len();

    let mut handle = BufWriter::new(io::stdout());
    let mut output = String::new();

    output.push('\n');
    output.push_str(&format!(
        "Found {} file/s which match file regex.\n",
        found_matches_count
    ));
    for (f, r) in results {
        output.push_str(&format!("Filename found with matches: {}\n", f));
        for m in r {
            output.push_str(&m.to_string());
            output.push('\n');
        }
    }
    handle
        .write_all(output.as_bytes())
        .map_err(MyErrors::FileIO)?;
    Ok(())
}

fn use_single_thread<I>(iterator: I, re: &Regex, print: bool) -> Result<(), MyErrors>
where
    I: Iterator<Item = Result<FileInfo, MyErrors>>,
{
    let results: Vec<(String, Vec<String>)> = iterator
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
            Ok(found) => {
                if !found.is_empty() {
                    Some((file.get_identifier(), found))
                } else {
                    None
                }
            }
        })
        .collect();

    if print {
        information_out(&results)?;
    }

    Ok(())
}

/**
 * This is the initial implementation using thread::spawn
 */
fn use_thread_per_file<I>(iterator: I, re: &Regex, print: bool) -> Result<(), MyErrors>
where
    I: Iterator<Item = Result<FileInfo, MyErrors>>,
{
    let matched_paths = iterator.filter_map(|r| r.ok()).collect::<Vec<FileInfo>>();

    let mut handles = Vec::new();
    let re = Arc::new(re.to_owned());
    for file in matched_paths {
        let re: Arc<Regex> = Arc::clone(&re);
        let file_id = file.get_identifier();
        let handle: thread::JoinHandle<Vec<String>> =
            thread::spawn(move || match find_entry_within_file(&file, &re) {
                Err(err) => {
                    eprintln!("Error while searching file {}", err);
                    Vec::new()
                }
                Ok(found) => found,
            });

        handles.push((file_id, handle));
    }

    let results = handles
        .into_iter()
        .map(|f| (f.0, f.1.join().unwrap()))
        .filter(|result| !result.1.is_empty())
        .collect::<Vec<(String, _)>>();

    if print {
        information_out(&results)?;
    }

    Ok(())
}

fn use_thread_pool<I>(
    iterator: I,
    re: &Regex,
    print: bool,
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
        let file_id = file.get_identifier();

        pool.execute(move || match find_entry_within_file(&file, &re) {
            Err(err) => {
                eprintln!("Error while searching file {}", err);
                tx.send((file_id, Vec::new()))
                    .expect("Critical error when handling error on file internal search");
            }
            Ok(found) => {
                tx.send((file_id, found))
                    .expect("Critical error while handling successful file internal searc");
            }
        });
    }
    drop(tx);

    let results: Vec<_> = rx
        .iter()
        .take(number_of_jobs)
        .filter(|results| !results.1.is_empty())
        .collect();
    pool.join();

    if print {
        information_out(&results)?;
    }

    Ok(())
}

fn use_rayon<I>(iterator: I, re: &Regex, print: bool) -> Result<(), MyErrors>
where
    I: ParallelIterator<Item = Result<FileInfo, MyErrors>>,
{
    let re = Arc::new(re.to_owned());
    let results: Vec<_> = iterator
        .filter_map(|item| match item {
            Ok(file) => Some(file),
            Err(err) => {
                eprintln!("Error parsing item: {}", err);
                None
            }
        })
        .filter_map(|file| {
            let re: Arc<Regex> = Arc::clone(&re);
            match find_entry_within_file_rayon(&file, &re) {
                Err(err) => {
                    eprintln!("Error while searching file {}", err);
                    None
                }
                Ok(found) => {
                    if !found.is_empty() {
                        Some((file.get_identifier(), found))
                    } else {
                        None
                    }
                }
            }
        })
        .collect();

    if print {
        information_out(&results)?;
    }

    Ok(())
}

fn find_files(dir: &Path, re: &Regex) -> impl Iterator<Item = Result<FileInfo, MyErrors>> {
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

/**
 * Theres definitely room for improvement here.
 * This is called by "all" the different functions and is entirely sequential not taking advantage of
 * all concurrency/parallelism.
 * TODO: either expand on this OR more likely make separate ones (in particular for Rayon)
 */
fn find_entry_within_file(f: &FileInfo, re: &Regex) -> Result<Vec<String>, MyErrors> {
    let file = File::open(&f.path).map_err(MyErrors::FileIO)?;
    let reader = BufReader::new(file);

    let mut found_lines = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line.map_err(MyErrors::FileIO)?;

        let replaced = re.replace_all(&line, |caps: &regex::Captures| {
            Colour::Red.paint(&caps[0]).to_string()
        });

        if replaced != line {
            found_lines.push(format!("Line {} - {}", idx + 1, replaced));
        }
    }

    Ok(found_lines)
}

fn find_entry_within_file_rayon(f: &FileInfo, re: &Regex) -> Result<Vec<String>, MyErrors> {
    let file = File::open(&f.path).map_err(MyErrors::FileIO)?;
    let reader = BufReader::with_capacity(8 * 1024, file);

    let mut results: Vec<(usize, String)> = reader
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .map_err(MyErrors::FileIO)?
        .into_par_iter()
        .enumerate()
        .filter_map(|(idx, line)| {
            let replaced = re.replace_all(&line, |caps: &regex::Captures| {
                let matched = &caps[0];
                let coloured = Colour::Red.paint(matched);
                coloured.to_string()
            });

            // equivalent to:
            if let Cow::Owned(_) = replaced {
                Some((idx, format!("Line {} - {}", idx + 1, replaced)))
            } else {
                None
            }
        })
        .collect();

    results.par_sort_unstable_by_key(|(idx, _)| *idx);
    Ok(results.into_iter().map(|(_, f)| f).collect())
}
