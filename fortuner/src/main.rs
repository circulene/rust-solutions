use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::exit,
};

use anyhow::Result;
use clap::Parser;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use regex::RegexBuilder;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(version, author, about)]
pub struct Args {
    /// Input files or directories
    #[arg(value_name = "FILES", required = true)]
    sources: Vec<String>,

    /// Pattern
    #[arg(short = 'm', long = "pattern", value_name = "PATTERN")]
    pattern_str: Option<String>,

    /// Case-insensitive pattern matching
    #[arg(short = 'i', long = "insensitive")]
    insensitive: bool,

    /// Random seed
    #[arg(short = 's', long = "seed", value_name = "SEED")]
    seed: Option<u64>,
}

#[derive(Debug)]
pub struct Fortune {
    source: String,
    text: String,
}

fn find_files(paths: &[String]) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = Vec::new();
    for dentry in paths.iter().flat_map(WalkDir::new) {
        let dentry = dentry?;
        if dentry.file_type().is_file() {
            files.push(dentry.into_path());
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn read_fortunes(paths: &[PathBuf]) -> Result<Vec<Fortune>> {
    let mut fortunes: Vec<Fortune> = Vec::new();
    for path in paths {
        let mut file = BufReader::new(File::open(path)?);
        let mut line = String::new();
        let mut text = String::new();
        while file.read_line(&mut line)? > 0 {
            if line.starts_with('%') {
                let trimmed_text = text.trim_end();
                if !trimmed_text.is_empty() {
                    fortunes.push(Fortune {
                        source: path.to_string_lossy().to_string(),
                        text: trimmed_text.to_string(),
                    });
                }
                text.clear();
            } else {
                text.push_str(line.as_str());
            }
            line.clear();
        }
    }
    Ok(fortunes)
}

fn pick_fortune(fortunes: &[Fortune], seed: Option<u64>) -> Option<String> {
    match seed {
        Some(seed) => {
            let mut rng = StdRng::seed_from_u64(seed);
            fortunes.choose(&mut rng)
        }
        None => {
            let mut rng = rand::thread_rng();
            fortunes.choose(&mut rng)
        }
    }
    .map(|f| f.text.to_owned())
}

fn run() -> Result<()> {
    let args = Args::parse();
    let pattern = args
        .pattern_str
        .map(|pattern| {
            RegexBuilder::new(&pattern)
                .case_insensitive(args.insensitive)
                .build()
        })
        .transpose()?;
    let files = find_files(&args.sources)?;
    let fortunes = read_fortunes(&files)?;
    println!("{}", fortunes.last().unwrap().text);
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_files() {
        let res = find_files(&["./tests/inputs/jokes".to_string()]);
        assert!(res.is_ok());

        let files = res.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files.first().unwrap().to_string_lossy(),
            "./tests/inputs/jokes"
        );

        let res = find_files(&["/path/does/not/exist".to_string()]);
        assert!(res.is_err());

        let res = find_files(&["./tests/inputs".to_string()]);
        assert!(res.is_ok());

        let files = res.unwrap();
        assert_eq!(files.len(), 5);
        let first = files.first().unwrap().display().to_string();
        assert!(first.contains("ascii-art"));
        let last = files.last().unwrap().display().to_string();
        assert!(last.contains("quotes"));

        let res = find_files(&[
            "./tests/inputs/jokes".to_string(),
            "./tests/inputs/ascii-art".to_string(),
            "./tests/inputs/jokes".to_string(),
        ]);
        assert!(res.is_ok());
        let files = res.unwrap();
        assert_eq!(files.len(), 2);
        if let Some(filename) = files.first().unwrap().file_name() {
            assert_eq!(filename.to_string_lossy(), "ascii-art".to_string());
        }
        if let Some(filename) = files.last().unwrap().file_name() {
            assert_eq!(filename.to_string_lossy(), "jokes".to_string());
        }
    }

    #[test]
    fn test_read_fortunes() {
        let res = read_fortunes(&[PathBuf::from("./tests/inputs/jokes")]);
        assert!(res.is_ok());

        if let Ok(fortunes) = res {
            assert_eq!(fortunes.len(), 6);
            assert_eq!(
                fortunes.first().unwrap().text,
                "Q. What do you call a head of lettuce in a shirt and tie?\n\
                A. Collared greens."
            );
            assert_eq!(
                fortunes.last().unwrap().text,
                "Q: What do you call a deer wearing an eye patch?\n\
                A: A bad idea (bad-eye deer)."
            );
        }

        let res = read_fortunes(&[
            PathBuf::from("./tests/inputs/jokes"),
            PathBuf::from("./tests/inputs/quotes"),
        ]);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().len(), 11);
    }

    #[test]
    fn test_pick_fortune() {
        let fortunes = [
            Fortune {
                source: "fortune".to_string(),
                text: "This is a pen.".to_string(),
            },
            Fortune {
                source: "fortune".to_string(),
                text: "This is an apple.".to_string(),
            },
            Fortune {
                source: "fortune".to_string(),
                text: "This is a pineapple.".to_string(),
            },
        ];
        assert_eq!(
            pick_fortune(&fortunes, Some(1)).unwrap(),
            "This is a pineapple.".to_string()
        );
    }
}
