use std::{os::macos::fs::MetadataExt, path::PathBuf, process::exit};

use anyhow::{Error, Result};
use chrono::DateTime;
use clap::Parser;
use tabular::{Row, Table};

static PERM_READABLE: u32 = 0b100;
static PERM_WRITEABLE: u32 = 0b010;
static PERM_EXECUTABLE: u32 = 0b001;

static MODE_MASK: u32 = 0b111;
static MODE_USER_SHIFT: u32 = 6;
static MODE_GROUP_SHIFT: u32 = 3;

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
    let path_error = |path: &PathBuf, e| {
        let context = format!("{}: {}", path.display(), e);
        Error::new(e).context(context)
    };
    for path in paths {
        let path = PathBuf::from(path);
        if path.metadata().map_err(|e| path_error(&path, e))?.is_dir() {
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
    Ok(files)
}

fn format_output(paths: &[PathBuf]) -> Result<String> {
    //               1   2     3     4     5     6     7     8
    let fmt = "{:<}{:<}  {:>}  {:<}  {:<}  {:>}  {:<}  {:<}";
    let mut table = Table::new(fmt);

    for path in paths {
        let metadata = path.metadata()?;
        table.add_row(
            Row::new()
                // 1 "d" or "-"
                .with_cell(if metadata.is_dir() { "d" } else { "-" })
                // 2 permission
                .with_cell(format_mode(metadata.st_mode()))
                // 3 number of links
                .with_cell(metadata.st_nlink())
                // 4 user name
                .with_cell(
                    users::get_user_by_uid(metadata.st_uid())
                        .map(|u| u.name().to_string_lossy().to_string())
                        .unwrap_or(metadata.st_uid().to_string()),
                )
                // 5 group name
                .with_cell(
                    users::get_group_by_gid(metadata.st_gid())
                        .map(|g| g.name().to_string_lossy().to_string())
                        .unwrap_or(metadata.st_gid().to_string()),
                )
                // 6 size
                .with_cell(metadata.st_size())
                // 7 modified time
                .with_cell(
                    DateTime::from_timestamp_nanos(metadata.st_mtime_nsec())
                        .format("%Y/%m/%d %H:%M"),
                )
                // 8 path
                .with_cell(path.display()),
        );
    }

    Ok(format!("{}", table))
}

fn format_mode(mode: u32) -> String {
    let bit_to_sym = |target: u32, bit: u32, sym: &str| {
        if target & bit != 0 {
            sym.to_string()
        } else {
            '-'.to_string()
        }
    };
    let stringify_mode = |m: u32| {
        [
            bit_to_sym(m, PERM_READABLE, "r"),
            bit_to_sym(m, PERM_WRITEABLE, "w"),
            bit_to_sym(m, PERM_EXECUTABLE, "x"),
        ]
        .join("")
    };
    [
        stringify_mode((mode >> MODE_USER_SHIFT) & MODE_MASK),
        stringify_mode((mode >> MODE_GROUP_SHIFT) & MODE_MASK),
        stringify_mode(mode & MODE_MASK),
    ]
    .join("")
}

fn run(args: &Args) -> Result<()> {
    for path in find_files(&args.paths, args.show_hidden)? {
        println!("{}", path.display());
    }
    Ok(())
}

fn main() {
    let args = Args::parse();
    if let Err(e) = run(&args) {
        eprintln!("{}", e);
        exit(0);
    }
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

    #[test]
    fn test_format_mode() {
        assert_eq!(format_mode(0o755), "rwxr-xr-x");
        assert_eq!(format_mode(0o421), "r---w---x");
    }
}
