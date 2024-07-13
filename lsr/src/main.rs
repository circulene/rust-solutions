use std::{os::macos::fs::MetadataExt, path::PathBuf, process::exit};

use anyhow::{Error, Result};
use chrono::{DateTime, Local};
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
                .with_cell(DateTime::<Local>::from(metadata.modified()?).format("%b %d %y %H:%M"))
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
            "-".to_string()
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
    let paths = find_files(&args.paths, args.show_hidden)?;
    if args.long {
        print!("{}", format_output(&paths)?);
    } else {
        for path in &paths {
            println!("{}", path.display());
        }
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

    fn long_match(
        line: &str,
        expected_name: &str,
        expected_perms: &str,
        expected_size: Option<&str>,
    ) {
        let parts: Vec<_> = line.split_whitespace().collect();
        assert!(parts.len() > 0 && parts.len() <= 10);

        let perms = parts.get(0).unwrap();
        assert_eq!(perms, &expected_perms);

        if let Some(size) = expected_size {
            let file_size = parts.get(4).unwrap();
            assert_eq!(file_size, &size);
        }

        let display_name = parts.last().unwrap();
        assert_eq!(display_name, &expected_name);
    }

    #[test]
    fn test_format_output_one() {
        let bustle_path = "tests/inputs/bustle.txt";
        let bustle = PathBuf::from(bustle_path);

        let res = format_output(&[bustle]);
        assert!(res.is_ok());

        let out = res.unwrap();
        let lines: Vec<&str> = out.split("\n").filter(|s| !s.is_empty()).collect();
        assert_eq!(lines.len(), 1);

        let line1 = lines.first().unwrap();
        long_match(line1, bustle_path, "-rw-r--r--", Some("193"));
    }

    #[test]
    fn test_format_output_two() {
        let res = format_output(&[
            PathBuf::from("tests/inputs/dir"),
            PathBuf::from("tests/inputs/empty.txt"),
        ]);
        assert!(res.is_ok());

        let out = res.unwrap();
        let mut lines: Vec<&str> = out.split("\n").filter(|s| !s.is_empty()).collect();
        lines.sort();
        assert_eq!(lines.len(), 2);

        let empty_line = lines.remove(0);
        long_match(
            empty_line,
            "tests/inputs/empty.txt",
            "-rw-r--r--",
            Some("0"),
        );
    }
}
