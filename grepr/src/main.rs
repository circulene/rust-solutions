use anyhow::{Error, Result};
use clap::{command, Parser};
use regex::RegexBuilder;
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

fn run(args: Args) -> Result<()> {
    dbg!(&args);
    let pattern = RegexBuilder::new(&args.pattern)
        .case_insensitive(args.insensitive)
        .build()
        .map_err(|err| Error::msg(format!("Invalid pattern \"{}\"", &args.pattern)))?;
    for path in find_files(&args.files, args.recursive) {
        println!("{}", path?);
    }
    Ok(())
}

fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("{}", e);
    }
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{distributions::Alphanumeric, Rng};

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
}
