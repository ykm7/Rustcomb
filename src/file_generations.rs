use assert_fs::{fixture, prelude::*};
use std::{
    env, fmt,
    path::{Path, PathBuf},
    str::FromStr,
};

const LIGHT_FILE: &str = "light_file.txt";
const MEDIUM_FILE: &str = "medium_file.txt";
const HEAVY_FILE: &str = "heavy_file.txt";
const TEST_FILE_DIRECTORY: &str = "test_files";

pub enum FileType {
    Light,
    Medium,
    Heavy,
}

impl fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileType::Light => write!(f, "Light"),
            FileType::Medium => write!(f, "Medium"),
            FileType::Heavy => write!(f, "Heavy"),
        }
    }
}

impl FromStr for FileType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "light" => Ok(FileType::Light),
            "medium" => Ok(FileType::Medium),
            "heavy" => Ok(FileType::Heavy),
            _ => Err(
                "Not an expected conversion string. Required to be either [light, medium or heavy]"
                    .to_string(),
            ),
        }
    }
}

impl FileType {
    fn get_filename(&self) -> &'static str {
        match self {
            FileType::Light => LIGHT_FILE,
            FileType::Medium => MEDIUM_FILE,
            FileType::Heavy => HEAVY_FILE,
        }
    }

    fn get_path(&self) -> PathBuf {
        let project_root = match find_project_root() {
            None => panic!("Unable to find root directory"),
            Some(dir) => dir,
        };

        let target_dir = project_root.join(TEST_FILE_DIRECTORY);
        if !target_dir.is_dir() {
            panic!("Project path isn't a directory!")
        } else {
            target_dir.join(self.get_filename())
        }
    }
}

pub fn create_files(
    temp: &fixture::TempDir,
    file_to_duplicate: FileType,
    num_of_nested_dirs: usize,
    num_of_files_to_create: usize,
) -> &Path {
    let _ = num_of_nested_dirs;
    // let temp = assert_fs::TempDir::new().unwrap();
    let filename = file_to_duplicate.get_filename();
    let filename_path = file_to_duplicate.get_path();

    for idx in 1..=num_of_files_to_create {
        temp.child(format!("{}_{}", idx, filename))
            .write_file(&filename_path)
            .unwrap();
    }

    temp.path()
}

fn find_project_root() -> Option<PathBuf> {
    let mut current_dir = env::current_dir().ok()?;
    loop {
        if current_dir.join("Cargo.toml").exists() {
            return Some(current_dir);
        }
        if !current_dir.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use walkdir::WalkDir;

    use super::*;

    #[test]
    fn test_setup() {
        let num_of_files_to_duplicate = 5;
        let num_of_directories_to_duplicate = 1;

        let temp_dir: fixture::TempDir = assert_fs::TempDir::new().unwrap();
        let p = create_files(
            &temp_dir,
            FileType::Light,
            num_of_directories_to_duplicate,
            num_of_files_to_duplicate,
        );
        assert!(
            p.is_dir(),
            "This should return the directory where the test files are available"
        );

        // purely for testing.
        let available_files = WalkDir::new(p)
            .into_iter()
            .filter(|entry| match entry {
                Err(err) => {
                    panic!("{}", err);
                }
                Ok(entry) => !entry.file_type().is_dir(),
            })
            .inspect(|entry| {
                println!("Entry is: {:?}", entry);
            })
            .count();

        assert_eq!(
            available_files,
            num_of_files_to_duplicate * num_of_directories_to_duplicate,
            "We should find the number of files including the nested directories."
        );

        temp_dir.close().unwrap();
    }
}
