use ansi_term::Colour;
use clap::Parser;
use core::fmt;
use futures::TryStreamExt;
use futures::stream::{self, StreamExt};
use memmap2::MmapOptions;
use my_regex::SearchMode;
use rayon::prelude::*;
use regex::Regex;
use regex::bytes;
use std::borrow::Cow;
use std::error;
use std::error::Error;
use std::fmt::Display;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::StdoutLock;
use std::io::{self, Write};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::PoisonError;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::thread;
use threadpool::ThreadPool;
use tokio::io::AsyncReadExt;
use walkdir::WalkDir;

pub mod my_regex;
/// This trait (and its implementation) are more to experiment with this behaviour rather than
/// an required bit of functionality.
/// However it should result "logic" shifting from runtime to compile-time so should be beneficial too.
pub trait Printable: Send + 'static + Copy + Clone {
    fn writeln<F>(&self, data: Vec<(String, Vec<String>)>, func: F) -> Result<(), MyErrors>
    where
        F: FnOnce(Vec<(String, Vec<String>)>) -> Result<(), MyErrors>;
    fn writeln_w_handler<T, F>(&self, handler: &mut BufWriter<T>, func: F) -> Result<(), MyErrors>
    where
        T: std::io::Write,
        F: FnOnce(&mut BufWriter<T>) -> Result<(), MyErrors>;
    fn information_out<T, F>(
        &self,
        handler: &mut BufWriter<T>,
        data: (String, Vec<String>),
        func: F,
    ) -> Result<(), MyErrors>
    where
        T: std::io::Write,
        F: FnOnce(&mut BufWriter<T>, (String, Vec<String>)) -> Result<(), MyErrors>;
}

#[derive(Clone, Copy)]
pub struct PrintEnabled;
#[derive(Clone, Copy)]
pub struct PrintDisable;

impl fmt::Display for PrintEnabled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Print is enabled")
    }
}

impl fmt::Display for PrintDisable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Print is disabled")
    }
}

impl Printable for PrintEnabled {
    fn writeln<F>(&self, data: Vec<(String, Vec<String>)>, func: F) -> Result<(), MyErrors>
    where
        F: FnOnce(Vec<(String, Vec<String>)>) -> Result<(), MyErrors>,
    {
        func(data)
    }

    fn writeln_w_handler<T, F>(
        &self,
        handler: &mut BufWriter<T>,
        func: F,
    ) -> std::result::Result<(), MyErrors>
    where
        T: std::io::Write,
        F: FnOnce(&mut BufWriter<T>) -> Result<(), MyErrors>,
    {
        func(handler)
    }

    fn information_out<T, F>(
        &self,
        handler: &mut BufWriter<T>,
        data: (String, Vec<String>),
        func: F,
    ) -> Result<(), MyErrors>
    where
        T: std::io::Write,
        F: FnOnce(&mut BufWriter<T>, (String, Vec<String>)) -> Result<(), MyErrors>,
    {
        func(handler, data)
    }
}

impl Printable for PrintDisable {
    fn writeln<F>(&self, _: Vec<(String, Vec<String>)>, _: F) -> Result<(), MyErrors>
    where
        F: FnOnce(Vec<(String, Vec<String>)>) -> Result<(), MyErrors>,
    {
        Ok(())
    }

    fn writeln_w_handler<T, F>(
        &self,
        _: &mut BufWriter<T>,
        _: F,
    ) -> std::result::Result<(), MyErrors>
    where
        T: std::io::Write,
        F: FnOnce(&mut BufWriter<T>) -> Result<(), MyErrors>,
    {
        Ok(())
    }

    fn information_out<T, F>(
        &self,
        _: &mut BufWriter<T>,
        _: (String, Vec<String>),
        _: F,
    ) -> Result<(), MyErrors>
    where
        T: std::io::Write,
        F: FnOnce(&mut BufWriter<T>, (String, Vec<String>)) -> Result<(), MyErrors>,
    {
        Ok(())
    }
}

#[derive(Debug)]
pub enum MyErrors {
    Regex(regex::Error),
    WalkDir(walkdir::Error),
    FileIO(io::Error),
    Utf8Error(std::string::FromUtf8Error),
    LockError(String),
    ThreadPanic(String),
    SomeError(String),
    TokioError(tokio::task::JoinError),
}

impl fmt::Display for MyErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            MyErrors::Regex(ref e) => write!(f, "regex error: ({})", e),
            MyErrors::WalkDir(ref e) => write!(f, "WalkDir error: ({})", e),
            MyErrors::FileIO(ref e) => write!(f, "File IO error: ({})", e),
            MyErrors::LockError(ref e) => write!(f, "Lock error ({})", e),
            MyErrors::ThreadPanic(ref e) => write!(f, "thread error ({})", e),
            MyErrors::SomeError(ref e) => write!(f, "value expected to be not None ({})", e),
            MyErrors::Utf8Error(ref e) => write!(f, "UTF8 error ({})", e),
            MyErrors::TokioError(ref e) => write!(f, "TokioError error ({})", e),
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
            MyErrors::ThreadPanic(_) => None,
            MyErrors::SomeError(_) => None,
            MyErrors::Utf8Error(ref e) => Some(e),
            MyErrors::TokioError(ref e) => Some(e),
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
// TODO: Add short flag support
pub struct Cli {
    /// The directory to search within
    pub path: std::path::PathBuf,

    /// Pattern matching mode for within the file
    #[clap(short, long, default_value="literal", value_name = "MODE", value_parser = clap::builder::EnumValueParser::<SearchMode>::new(), )]
    pub file_pattern_regex: SearchMode,

    /// The file internal pattern to look for
    pub file_pattern: String,

    /// Pattern matching mode for filenames
    #[clap(short, long, default_value="literal", value_name = "MODE", value_parser = clap::builder::EnumValueParser::<SearchMode>::new(),)]
    pub path_pattern_regex: SearchMode,

    /// The file name pattern to look for
    pub path_pattern: Option<String>,
}

impl std::fmt::Display for Cli {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Path: {:?}, File pattern: {}, Path pattern: {:?}",
            self.path, self.file_pattern, self.path_pattern
        )
    }
}

pub fn get_cpuworkers() -> usize {
    std::thread::available_parallelism().map_or(4, |n| n.get())
}

#[inline]
pub fn single_thread_read_files<P: Printable>(
    args: Arc<Cli>,
    print_behaviour: P,
) -> Result<(), MyErrors> {
    let path_pattern =
        my_regex::clean_up_regex(args.path_pattern.as_deref(), args.path_pattern_regex)?;
    let iterator = find_files(&args.path, path_pattern);
    let file_pattern_re =
        my_regex::clean_up_regex(Some(&args.file_pattern), args.file_pattern_regex)?.ok_or(
            MyErrors::SomeError("'file_pattern' is expected to exist".to_string()),
        )?;
    use_single_thread(iterator, &file_pattern_re, print_behaviour)?;
    Ok(())
}

#[inline]
pub fn rayon_read_files<P: Printable>(args: Arc<Cli>, print_behaviour: P) -> Result<(), MyErrors> {
    let path_pattern =
        my_regex::clean_up_regex(args.path_pattern.as_deref(), args.path_pattern_regex)?;
    let rayon_iterator = rayon_find_files(&args.path, path_pattern);
    let file_pattern_re =
        my_regex::clean_up_regex(Some(&args.file_pattern), args.file_pattern_regex)?.ok_or(
            MyErrors::SomeError("'file_pattern' is expected to exist".to_string()),
        )?;
    use_rayon(rayon_iterator, &file_pattern_re, print_behaviour)?;

    Ok(())
}

#[inline]
pub fn thread_per_file_read_files<P: Printable>(
    args: Arc<Cli>,
    print_behaviour: P,
) -> Result<(), MyErrors> {
    let path_pattern =
        my_regex::clean_up_regex(args.path_pattern.as_deref(), args.path_pattern_regex)?;
    let iterator = find_files(&args.path, path_pattern);
    let file_pattern_re =
        my_regex::clean_up_regex(Some(&args.file_pattern), args.file_pattern_regex)?.ok_or(
            MyErrors::SomeError("'file_pattern' is expected to exist".to_string()),
        )?;
    use_thread_per_file(iterator, &file_pattern_re, print_behaviour)?;

    Ok(())
}

#[inline]
pub fn threadpool_read_files<P: Printable>(
    args: Arc<Cli>,
    print_behaviour: P,
    number_of_workers: usize,
) -> Result<(), MyErrors> {
    let path_pattern =
        my_regex::clean_up_regex(args.path_pattern.as_deref(), args.path_pattern_regex)?;
    let iterator = find_files(&args.path, path_pattern);
    let file_pattern_re =
        my_regex::clean_up_regex(Some(&args.file_pattern), args.file_pattern_regex)?.ok_or(
            MyErrors::SomeError("'file_pattern' is expected to exist".to_string()),
        )?;
    use_thread_pool::<_, _, { 256 * 1024 }>(
        iterator,
        &file_pattern_re,
        print_behaviour,
        number_of_workers,
    )?;

    Ok(())
}

/// TODO: examine iterator, likely add async friendly iterator instead of forcing existing to work.
pub async fn async_read_files<P: Printable>(
    args: Arc<Cli>,
    print_behaviour: P,
) -> Result<(), MyErrors> {
    let path_pattern =
        my_regex::clean_up_regex(args.path_pattern.as_deref(), args.path_pattern_regex)?;
    let iterator = find_files(&args.path, path_pattern);
    let file_pattern_re =
        my_regex::clean_up_regex(Some(&args.file_pattern), args.file_pattern_regex)?.ok_or(
            MyErrors::SomeError("'file_pattern' is expected to exist".to_string()),
        )?;
    use_async_two::<_, _>(iterator, &file_pattern_re, print_behaviour).await?;

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

// /**
//  * Use a single initialised re pattern to save it being created on each call (STAR_PATTERN)
//  *
//  * re.Replace() returns a COW (Copy on Write) type.
//  * So a new string is only created when its modified.
//  * Can be used whether using the Borrowed or the Owned
//  */
// fn clean_up_regex(pattern: Option<&str>) -> Result<Option<regex::Regex>, MyErrors> {
//     pattern
//         .map(|pat| {
//             let escaped = regex::escape(pat);
//             let cleaned: std::borrow::Cow<'_, str> = STAR_PATTERN.replace(&escaped, ".*");
//             Regex::new(&cleaned).map_err(MyErrors::Regex)
//         })
//         .transpose()
// }

fn information_out(results: &Vec<(String, Vec<String>)>) -> Result<(), MyErrors> {
    let found_matches_count = results.len();

    let mut handle = BufWriter::new(io::stdout());
    let mut output = String::new();

    output.push('\n');
    output.push_str(&format!("Found {} files\n", found_matches_count));
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

/// Based on various suggestions - Matches common filesystem block sizes // 64KB
fn information_out_each_lock_default(
    handle: &mut BufWriter<StdoutLock>,
    results: &(String, Vec<String>),
) -> Result<(), MyErrors> {
    information_out_each_lock::<{ 64 * 1024 }>(handle, results)
}

fn information_out_each_lock<const FLUSH_THRESHOLD: usize>(
    handle: &mut BufWriter<StdoutLock>,
    results: &(String, Vec<String>),
) -> Result<(), MyErrors> {
    let (file, r) = results;
    writeln!(handle, "Filename found with matches: {}", file).map_err(MyErrors::FileIO)?;
    for m in r {
        writeln!(handle, "{}", m).map_err(MyErrors::FileIO)?;
    }
    // periodic flushing.
    if handle.buffer().len() > FLUSH_THRESHOLD {
        handle.flush().map_err(MyErrors::FileIO)?;
    }

    Ok(())
}

async fn use_async_two<I, P: Printable>(
    iterator: I,
    re: &Regex,
    print_behaviour: P,
) -> Result<(), MyErrors>
where
    I: Iterator<Item = FileInfo>,
{
    let results = stream::iter(iterator)
        .map(|f: FileInfo| {
            let identifier = f.get_identifier();
            let path = f.path.clone();
            let re_copy = re.clone();

            async move {
                let buffer = tokio::fs::read(path).await.map_err(MyErrors::FileIO)?;

                let found: Vec<String> = tokio::task::spawn_blocking(
                    // useful when expecting a task/s which ARE CPU bound
                    move || -> Result<Vec<String>, MyErrors> {
                        let contents = String::from_utf8(buffer).map_err(MyErrors::Utf8Error)?;
                        let found = contents
                            .lines()
                            .enumerate()
                            .filter_map(|(idx, line)| -> Option<String> {
                                let replaced =
                                    re_copy.replace_all(line, |caps: &regex::Captures| {
                                        Colour::Red.paint(&caps[0]).to_string()
                                    });

                                if let Cow::Owned(_) = replaced {
                                    Some(format!(
                                        "{}:{}",
                                        Colour::Green.paint(format!("{}", idx + 1)),
                                        replaced
                                    ))
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<String>>();

                        Ok(found)
                    },
                )
                .await
                .map_err(MyErrors::TokioError)??;

                Ok::<(String, Vec<String>), MyErrors>((identifier, found))
            }
        })
        .buffer_unordered(get_cpuworkers()) // controls memory usage by limiting concurrency to something the system can handle
        .try_filter_map(|result: (String, Vec<String>)| async move {
            match result {
                (id, found) if !found.is_empty() => Ok(Some((id, found))),
                _ => Ok(None),
            }
        })
        .try_collect::<Vec<(String, Vec<String>)>>()
        .await?;

    print_behaviour.writeln(results, |r| information_out(&r))?;
    Ok(())
}

#[allow(dead_code)]
async fn use_async<I, P: Printable>(
    iterator: I,
    re: &Regex,
    print_behaviour: P,
) -> Result<(), MyErrors>
where
    I: Iterator<Item = FileInfo>,
{
    let results: Vec<(String, Vec<String>)> = stream::iter(iterator)
        .filter_map(|file| async move {
            find_entry_with_file_async(&file, re)
                .await
                .map_err(|err| {
                    eprintln!("Error while searching file {}", err);
                    err
                })
                .ok()
                .filter(|found| !found.is_empty())
                .map(|found| (file.get_identifier(), found))
        })
        .collect()
        .await;

    print_behaviour.writeln(results, |r| information_out(&r))?;

    Ok(())
}

#[allow(dead_code)]
async fn find_entry_with_file_async(f: &FileInfo, re: &Regex) -> Result<Vec<String>, MyErrors> {
    let mut found_lines = Vec::new();

    let mut file = tokio::fs::File::open(&f.path)
        .await
        .map_err(MyErrors::FileIO)?;
    let mut buffer = Vec::new();

    file.read_to_end(&mut buffer)
        .await
        .map_err(MyErrors::FileIO)?;

    let contents = String::from_utf8(buffer).map_err(MyErrors::Utf8Error)?;

    for (idx, line) in contents.lines().enumerate() {
        let replaced = re.replace_all(line, |caps: &regex::Captures| {
            Colour::Red.paint(&caps[0]).to_string()
        });

        if let Cow::Owned(_) = replaced {
            found_lines.push(format!(
                "{}:{}",
                Colour::Green.paint(format!("{}", idx + 1)),
                replaced
            ));
        }
    }

    Ok(found_lines)
}

fn use_single_thread<I, P: Printable>(
    iterator: I,
    re: &Regex,
    print_behaviour: P,
) -> Result<(), MyErrors>
where
    I: Iterator<Item = FileInfo>,
{
    let results: Vec<(String, Vec<String>)> = iterator
        .filter_map(|file| match find_entry_with_file_memmap(&file, re) {
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

    print_behaviour.writeln(results, |r| information_out(&r))?;

    Ok(())
}

/**
 * This is the initial implementation using thread::spawn
 */
fn use_thread_per_file<I, P: Printable>(
    iterator: I,
    re: &Regex,
    print_behaviour: P,
) -> Result<(), MyErrors>
where
    I: Iterator<Item = FileInfo>,
{
    let matched_paths = iterator.collect::<Vec<FileInfo>>();

    let mut handles = Vec::new();
    let re = Arc::new(re.to_owned());
    for file in matched_paths {
        let re: Arc<Regex> = Arc::clone(&re);
        let file_id = file.get_identifier();
        let handle: thread::JoinHandle<Vec<String>> =
            thread::spawn(move || match find_entry_with_file_memmap(&file, &re) {
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

    print_behaviour.writeln(results, |r| information_out(&r))?;

    Ok(())
}

///
/// Fits in L2 cache (most modern CPUs)
///
fn use_thread_pool<I, P: Printable, const BUF_CAPACITY: usize>(
    iterator: I,
    re: &Regex,
    print_behaviour: P,
    number_of_workers: usize,
) -> Result<(), MyErrors>
where
    I: Iterator<Item = FileInfo>,
{
    let pool = ThreadPool::new(number_of_workers);
    let re = Arc::new(re.to_owned());

    let (tx, rx) = crossbeam_channel::bounded(1000);
    let files_found_matching_file_regex = Arc::new(AtomicUsize::new(0));

    let print_handle = thread::spawn(move || -> Result<_, MyErrors> {
        let stdout = io::stdout();
        let mut handle = BufWriter::with_capacity(BUF_CAPACITY, stdout.lock()); // 1MB buffer

        print_behaviour
            .writeln_w_handler(&mut handle, |h| writeln!(h).map_err(MyErrors::FileIO))?;

        while let Ok(x) = rx.recv() {
            // TODO: Investigate the "Ordering" stuff more.
            files_found_matching_file_regex.fetch_add(1, Ordering::Relaxed);
            print_behaviour.information_out(&mut handle, x, |h, xx| {
                information_out_each_lock_default(h, &xx)
            })?;
        }

        print_behaviour.writeln_w_handler(&mut handle, |h: &mut BufWriter<StdoutLock<'_>>| {
            writeln!(
                h,
                "Found {} files",
                files_found_matching_file_regex.load(Ordering::Relaxed)
            )
            .map_err(MyErrors::FileIO)
        })?;

        handle.flush().map_err(MyErrors::FileIO)?;
        Ok(())
    });

    iterator.for_each(|file| {
        let tx: crossbeam_channel::Sender<(String, Vec<String>)> = tx.clone();
        let re: Arc<Regex> = Arc::clone(&re);
        let file_id = file.get_identifier();

        pool.execute(move || match find_entry_with_file_memmap(&file, &re) {
            Err(err) => {
                eprintln!("Error while searching file {}", err);
            }
            Ok(found) => {
                if !found.is_empty() {
                    if let Err(e) = tx.send((file_id, found)) {
                        eprintln!(
                            "Critical error while handling successful file internal search: {}",
                            e
                        )
                    }
                }
            }
        });
    });

    drop(tx);
    print_handle
        .join()
        .map_err(|err| MyErrors::ThreadPanic(format!("{:?}", err)))??;
    pool.join();

    Ok(())
}

fn use_rayon<I, P: Printable>(iterator: I, re: &Regex, print_behaviour: P) -> Result<(), MyErrors>
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

    print_behaviour.writeln(results, |r| information_out(&r))?;

    Ok(())
}

fn find_files(dir: &Path, re: Option<Regex>) -> impl Iterator<Item = FileInfo> {
    let iterator = WalkDir::new(dir).into_iter().filter_map(move |entry| {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                eprintln!("File/Dir error: {}", err);
                // Some(Err(MyErrors::WalkDir(err)));
                return None;
            }
        };

        if !entry.file_type().is_file() {
            return None;
        }

        let path = entry.path();
        let filename = path.file_name().and_then(|os_str| os_str.to_str());

        match filename {
            Some(filename) => {
                if let Some(re) = &re {
                    if re.is_match(filename) {
                        Some(FileInfo {
                            path: path.to_path_buf(),
                            filename: filename.to_string(),
                        })
                    } else {
                        None
                    }
                } else {
                    Some(FileInfo {
                        path: path.to_path_buf(),
                        filename: filename.to_string(),
                    })
                }
            }
            None => None,
        }
    });
    iterator
}

fn rayon_find_files(
    dir: &Path,
    re: Option<Regex>,
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
        .filter_map(move |entry| {
            let path = entry.path();
            let filename = path.file_name().and_then(|os_str| os_str.to_str());
            match filename {
                Some(filename) => {
                    if let Some(re) = &re {
                        if re.is_match(filename) {
                            Some(Ok(FileInfo {
                                path: path.to_path_buf(),
                                filename: filename.to_string(),
                            }))
                        } else {
                            None
                        }
                    } else {
                        Some(Ok(FileInfo {
                            path: path.to_path_buf(),
                            filename: filename.to_string(),
                        }))
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
#[allow(dead_code)]
fn find_entry_within_file(f: &FileInfo, re: &Regex) -> Result<Vec<String>, MyErrors> {
    let file = File::open(&f.path).map_err(MyErrors::FileIO)?;
    let reader = BufReader::new(file);

    let mut found_lines = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line.map_err(MyErrors::FileIO)?;

        let replaced = re.replace_all(&line, |caps: &regex::Captures| {
            Colour::Red.paint(&caps[0]).to_string()
        });

        if let Cow::Owned(_) = replaced {
            found_lines.push(format!(
                "{}:{}",
                Colour::Green.paint(format!("{}", idx + 1)),
                replaced
            ));
        }
    }

    Ok(found_lines)
}

fn find_entry_with_file_memmap(f: &FileInfo, re: &Regex) -> Result<Vec<String>, MyErrors> {
    let mut found_lines = Vec::new();
    let byte_re = bytes::Regex::new(re.as_str()).map_err(MyErrors::Regex)?;

    let file = File::open(&f.path).map_err(MyErrors::FileIO)?;

    // TODO: test .map vs .map_copy
    let mmap = unsafe { MmapOptions::new().map(&file).map_err(MyErrors::FileIO)? };

    let mut pos = 0;
    let mut line_num = 1;

    while pos < mmap.len() {
        let end = mmap[pos..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|p| pos + p)
            .unwrap_or(mmap.len());

        // Check for CRLF
        let is_crlf = end > pos && mmap[end - 1] == b'\r';
        let line_bytes = if is_crlf {
            &mmap[pos..end - 1] // Exclude \r
        } else {
            &mmap[pos..end]
        };

        // let line_bytes = &mmap[pos..end];
        let line_str = String::from_utf8_lossy(line_bytes);
        let mut current_pos = 0;

        let mut modified_line: Option<String> = None;

        for cap in byte_re.captures_iter(line_bytes) {
            if let Some(m) = cap.get(0) {
                let range = m.range();
                if range.start == range.end {
                    break;
                }

                let mat = modified_line.get_or_insert_with(|| line_str[..current_pos].to_string());
                mat.push_str(&line_str[current_pos..range.start]);
                mat.push_str(&format!(
                    "{}",
                    Colour::Red.paint(&line_str[range.start..range.end])
                ));

                current_pos = range.end;
            }
        }

        if let Some(mut m1) = modified_line {
            m1.push_str(&line_str[current_pos..]);
            found_lines.push(format!(
                "{}:{}",
                Colour::Green.paint(format!("{}", line_num)),
                m1
            ));
        }

        pos = if end < mmap.len() { end + 1 } else { end };
        line_num += 1;
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
                Some((
                    idx,
                    format!(
                        "{}:{}",
                        Colour::Green.paint(format!("{}", idx + 1)),
                        replaced
                    ),
                ))
            } else {
                None
            }
        })
        .collect();

    results.par_sort_unstable_by_key(|(idx, _)| *idx);
    Ok(results.into_iter().map(|(_, f)| f).collect())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::{
        FileInfo, find_entry_with_file_memmap,
        my_regex::{self, SearchMode},
    };

    #[test]
    fn test_find_entry_with_file_memmap_basic_regex() {
        let filename = "light_file.txt";
        let file_path: PathBuf = Path::new("test_files").join(filename);
        let file_info: FileInfo = FileInfo {
            filename: filename.to_string(),
            path: file_path,
        };

        let re = my_regex::clean_up_regex(
            Some("Dis dignissim pulvinar senectus at porta aenean."),
            SearchMode::Literal,
        )
        .expect("Expected to be able to create regex from string")
        .unwrap();

        let r = find_entry_with_file_memmap(&file_info, &re);

        let expected_results: [String; 1] = [format!(
            "{}:{}{}",
            ansi_term::Color::Green.paint("19"),
            "Rhoncus erat eros cubilia sociosqu amet vestibulum in. Convallis libero dolor nascetur penatibus sapien. Magna porttitor a mauris leo dictum fames at pulvinar. Condimentum enim feugiat sagittis torquent suscipit tempor commodo leo. Lacus enim curae penatibus nisi sapien duis in nostra. Dictum aliquet magna class gravida ante tempor ultricies. Nam taciti elit libero ornare per, laoreet auctor. ",
            ansi_term::Color::Red.paint("Dis dignissim pulvinar senectus at porta aenean.")
        )];
        let x = expected_results.to_vec();

        assert_eq!(r.ok().unwrap(), x)
    }

    #[test]
    fn test_find_entry_with_file_memmap_actually_using_regex() {
        let filename = "light_file.txt";
        let file_path: PathBuf = Path::new("test_files").join(filename);
        let file_info: FileInfo = FileInfo {
            filename: filename.to_string(),
            path: file_path,
        };

        let re = my_regex::clean_up_regex(
            Some("Dis[ ]dignissim[ ]pulvinar[ ]senectus[ ]at[ ]porta[ ]aenean."),
            SearchMode::Regex,
        )
        .expect("Expected to be able to create regex from string")
        .unwrap();

        let r = find_entry_with_file_memmap(&file_info, &re);

        let expected_results: [String; 1] = [format!(
            "{}:{}{}",
            ansi_term::Color::Green.paint("19"),
            "Rhoncus erat eros cubilia sociosqu amet vestibulum in. Convallis libero dolor nascetur penatibus sapien. Magna porttitor a mauris leo dictum fames at pulvinar. Condimentum enim feugiat sagittis torquent suscipit tempor commodo leo. Lacus enim curae penatibus nisi sapien duis in nostra. Dictum aliquet magna class gravida ante tempor ultricies. Nam taciti elit libero ornare per, laoreet auctor. ",
            ansi_term::Color::Red.paint("Dis dignissim pulvinar senectus at porta aenean.")
        )];
        let x = expected_results.to_vec();

        assert_eq!(r.ok().unwrap(), x)
    }
}
