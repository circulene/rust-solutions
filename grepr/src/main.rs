use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use anyhow::{Error, Result};
use clap::{command, Parser};
use regex::{Regex, RegexBuilder};
use walkdir::WalkDir;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(value_name = "PATTERN", help = "Search pattern")]
    pattern: String,

    #[arg(value_name = "FILE", help = "Input file(s)", default_values = ["-"])]
    files: Vec<String>,

    #[arg(short, long, help = "Recursive search")]
    recursive: bool,

    #[arg(short, long, help = "Count occurences")]
    count: bool,

    #[arg(short = 'v', long = "invert-match", help = "Invert match")]
    invert_match: bool,

    #[arg(short, long, help = "Case-insensitive")]
    insensitive: bool,
}

fn find_files(paths: &[String], recursive: bool) -> Vec<Result<String>> {
    if paths.len() == 1 && paths[0] == "-" {
        return vec![Ok("-".to_string())];
    }
    paths
        .iter()
        .flat_map(|path| WalkDir::new(path).max_depth(recursive as usize).into_iter())
        .map(|e| match e {
            Ok(e) => {
                if !recursive && e.file_type().is_dir() {
                    Err(Error::msg(format!(
                        "{} is a directory",
                        e.path().to_string_lossy()
                    )))
                } else {
                    Ok(e)
                }
            }
            Err(err) => Err(Error::new(err)),
        })
        .filter(|e| e.as_ref().map_or(true, |e| e.file_type().is_file()))
        .map(|e| e.map(|e| e.path().to_string_lossy().into_owned()))
        .collect::<Vec<_>>()
}

fn open(filename: &str) -> Result<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(std::io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

fn find_lines<T: BufRead>(mut file: T, pattern: &Regex, invert_match: bool) -> Result<Vec<String>> {
    let mut result = Vec::new();
    let mut buf = String::new();
    loop {
        match file.read_line(&mut buf) {
            Ok(0) => break,
            Ok(_) => {
                if pattern.is_match(&buf) {
                    if !invert_match {
                        result.push(buf.clone());
                    }
                } else if invert_match {
                    result.push(buf.clone());
                }
                buf.clear();
            }
            Err(e) => return Err(Error::new(e)),
        }
    }
    Ok(result)
}

fn run(args: Args) -> Result<()> {
    let pattern = RegexBuilder::new(&args.pattern)
        .case_insensitive(args.insensitive)
        .build()
        .map_err(|_| Error::msg(format!("Invalid pattern \"{}\"", &args.pattern)))?;
    let entries = find_files(&args.files, args.recursive);
    for entry in &entries {
        match entry {
            Err(e) => eprintln!("{}", e),
            Ok(filename) => match open(filename) {
                Err(e) => eprintln!("{}: {}", filename, e),
                Ok(file) => {
                    let matches = find_lines(file, &pattern, args.invert_match)?;
                    if args.count {
                        if entries.len() > 1 {
                            println!("{}:{}", filename, matches.len());
                        } else {
                            println!("{}", matches.len());
                        }
                    } else {
                        for line in matches {
                            if entries.len() > 1 {
                                print!("{}:{}", filename, line);
                            } else {
                                print!("{}", line);
                            }
                        }
                    }
                }
            },
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{distributions::Alphanumeric, Rng};
    use std::io::Cursor;

    #[test]
    fn test_find_files() {
        let files = find_files(&["./tests/inputs/fox.txt".to_string()], false);
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].as_ref().unwrap().to_owned(),
            "./tests/inputs/fox.txt".to_string()
        );

        let files = find_files(&["./tests/inputs".to_string()], false);
        assert_eq!(files.len(), 1);
        if let Err(e) = &files[0] {
            assert!(e.to_string().contains("./tests/inputs is a directory"));
        }

        let files = find_files(&["./tests/inputs".to_string()], true);
        let mut files: Vec<_> = files
            .iter()
            .map(|r| r.as_ref().unwrap().replace("\\", "/"))
            .collect();
        files.sort();
        assert_eq!(files.len(), 4);
        assert_eq!(
            files,
            vec![
                "./tests/inputs/bustle.txt",
                "./tests/inputs/empty.txt",
                "./tests/inputs/fox.txt",
                "./tests/inputs/nobody.txt"
            ]
        );

        let bad: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();

        let files = find_files(&[bad], false);
        assert_eq!(files.len(), 1);
        assert!(files[0].is_err());
    }

    #[test]
    fn test_find_lines() {
        let text = b"Lorem\nIpsum\r\nDOLOR";

        // should match "Lorem"
        let re1 = Regex::new("or").unwrap();
        let matches = find_lines(Cursor::new(&text), &re1, false);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 1);

        // should match "Ipsum" and "DOLOR"
        let matches = find_lines(Cursor::new(&text), &re1, true);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 2);

        // regex which does not distinguish sequence "or" from sequence "OR"
        let re2 = RegexBuilder::new("or")
            .case_insensitive(true)
            .build()
            .unwrap();

        // should match "Lorem" and "DOLOR"
        let matches = find_lines(Cursor::new(&text), &re2, false);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 2);

        // should match "Ipsum"
        let matches = find_lines(Cursor::new(&text), &re2, true);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 1);
    }
}
