use anyhow::Result;
use clap::Parser;
use std::{
    fmt::Debug,
    fs::File,
    io::{self, BufRead, BufReader},
};

#[derive(Parser, Debug)]
#[command(
    name = "wcr",
    version = "0.1.0",
    author = "circulene",
    about = "Rust wc"
)]
pub struct Config {
    /// Input file(s)
    #[arg(value_name = "FILE", default_value = "-")]
    files: Vec<String>,

    /// Show line count
    #[arg(short = 'l', long = "lines")]
    lines: bool,

    /// Show word count
    #[arg(short = 'w', long = "words")]
    words: bool,

    /// Show byte count
    #[arg(short = 'c', long = "bytes", conflicts_with = "chars")]
    bytes: bool,

    /// Show character count
    #[arg(short = 'm', long = "chars")]
    chars: bool,
}

#[derive(Debug, PartialEq)]
pub struct FileInfo {
    num_lines: usize,
    num_words: usize,
    num_bytes: usize,
    num_chars: usize,
}

impl FileInfo {
    fn new() -> FileInfo {
        FileInfo {
            num_lines: 0,
            num_words: 0,
            num_bytes: 0,
            num_chars: 0,
        }
    }

    fn add(&mut self, orig: &FileInfo) {
        self.num_lines += orig.num_lines;
        self.num_words += orig.num_words;
        self.num_bytes += orig.num_bytes;
        self.num_chars += orig.num_chars;
    }
}

pub fn get_args() -> Result<Config> {
    let args = Config::try_parse();
    match args {
        Ok(mut args) => {
            let no_flags = [args.lines, args.words, args.bytes, args.chars]
                .iter()
                .all(|v| v == &false);
            if no_flags {
                args = Config {
                    lines: true,
                    words: true,
                    bytes: true,
                    ..args
                }
            }
            Ok(args)
        }
        _ => Err(From::from(args.unwrap_err())),
    }
}

fn open(filename: &str) -> Result<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

pub fn count(mut file: impl BufRead) -> Result<FileInfo> {
    let mut num_lines = 0;
    let mut num_words = 0;
    let mut num_bytes = 0;
    let mut num_chars = 0;

    let mut line = String::new();
    loop {
        let read_byes = file.read_line(&mut line)?;
        if read_byes == 0 {
            break;
        }
        num_lines += 1;
        num_words += line.split_whitespace().count();
        num_bytes += read_byes;
        num_chars += line.chars().count();
        line.clear();
    }

    Ok(FileInfo {
        num_lines,
        num_words,
        num_bytes,
        num_chars,
    })
}

fn print_file_info(config: &Config, filename: &str, file_info: &FileInfo) {
    let mut counts: Vec<usize> = Vec::new();
    if config.lines {
        counts.push(file_info.num_lines);
    }
    if config.words {
        counts.push(file_info.num_words);
    }
    if config.bytes {
        counts.push(file_info.num_bytes);
    }
    if config.chars {
        counts.push(file_info.num_chars);
    }
    let result = counts
        .iter()
        .map(|n| format!("{:>8}", n))
        .fold(String::new(), |acc, x| format!("{acc}{x:>8}"));
    let show_file_name = if filename != "-" {
        format!(" {filename}")
    } else {
        "".to_string()
    };
    println!("{result}{show_file_name}");
}

pub fn run(config: Config) -> Result<()> {
    let mut total_file_info = FileInfo::new();
    for filename in &config.files {
        match open(filename) {
            Err(e) => eprintln!("{filename}: {e}"),
            Ok(file) => {
                let file_info = count(file)?;
                print_file_info(&config, filename, &file_info);
                total_file_info.add(&file_info);
            }
        }
    }
    if config.files.len() > 1 {
        print_file_info(&config, "total", &total_file_info);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{count, FileInfo};
    use std::io::Cursor;

    #[test]
    fn test_count() {
        let text = "I don't want the world. I just want your half.\r\n";
        let info = count(Cursor::new(text));
        assert!(info.is_ok());
        let expected = FileInfo {
            num_lines: 1,
            num_words: 10,
            num_chars: 48,
            num_bytes: 48,
        };
        assert_eq!(info.unwrap(), expected);
    }
}
