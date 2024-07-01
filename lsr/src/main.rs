use std::{path::PathBuf, process::exit};

use anyhow::{Error, Result};
use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Rust ls")]
struct Args {
    /// Files and/or directories
    #[arg(value_name = "PATH", default_value = ".")]
    paths: Vec<String>,

    /// Long listing
    #[arg(short = 'l', long = "long")]
    long: bool,

    /// show all files
    #[arg(short = 'a', long = "all")]
    show_hidden: bool,
}

fn find_files(paths: &[String], show_hidden: bool) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = vec![];
    for path in paths {
        let path = PathBuf::from(path);
        if path.exists() {
            if path.metadata()?.is_dir() {
                for entry in path.read_dir()? {
                    let entry = entry?;
                    if entry.file_name().to_string_lossy().starts_with('.') && !show_hidden {
                        continue;
                    }
                    files.push(entry.path());
                }
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}

fn run(args: &Args) -> Result<()> {
    Ok(())
}

fn main() {
    let args = Args::parse();
    if let Err(e) = run(&args) {
        eprintln!("{}", e);
        exit(1);
    }
    println!("Hello, world!");
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_find_files() {
        let res = find_files(&["tests/inputs".to_string()], false);
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            [
                "tests/inputs/bustle.txt",
                "tests/inputs/dir",
                "tests/inputs/empty.txt",
                "tests/inputs/fox.txt"
            ]
        );

        let res = find_files(&["tests/inputs/.hidden".to_string()], false);
        assert!(res.is_ok());
        let filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        assert_eq!(filenames, ["tests/inputs/.hidden"]);

        let res = find_files(
            &[
                "tests/inputs/bustle.txt".to_string(),
                "tests/inputs/dir".to_string(),
            ],
            false,
        );
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            ["tests/inputs/bustle.txt", "tests/inputs/dir/spiders.txt"]
        );
    }

    #[test]
    fn test_find_files_hidden() {
        let res = find_files(&["tests/inputs".to_string()], true);
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|entry| entry.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            [
                "tests/inputs/.hidden",
                "tests/inputs/bustle.txt",
                "tests/inputs/dir",
                "tests/inputs/empty.txt",
                "tests/inputs/fox.txt"
            ]
        );
    }
}
