use anyhow::{Error, Result};
use clap::Parser;
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Write},
};

#[derive(Parser, Debug)]
#[command(version, about = "Rust uniq")]
pub struct Config {
    /// Input file
    #[arg(value_name = "IN_FILE", default_value = "-")]
    in_file: String,

    /// Output file
    #[arg(value_name = "OUT_FILE")]
    out_file: Option<String>,

    /// Show counts
    #[arg(short = 'c', long = "count")]
    count: bool,
}

pub fn get_args() -> Result<Config> {
    let config = Config::try_parse()?;
    Ok(config)
}

fn open(filename: &str) -> Result<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

fn format(show_count: bool, counter: usize, line: &str) -> String {
    format!(
        "{}{}",
        if show_count {
            format!("{counter:>4} ")
        } else {
            "".to_string()
        },
        line
    )
}

pub fn run(config: Config) -> Result<()> {
    let mut file =
        open(&config.in_file).map_err(|e| Error::msg(format!("{}: {}", &config.in_file, e)))?;
    let mut out_file: Box<dyn Write> = match &config.out_file {
        Some(out_name) => Box::new(File::create(out_name)?),
        _ => Box::new(io::stdout()),
    };
    let mut line = String::new();
    let mut prev_line = None::<String>;
    let mut counter: usize = 0;
    loop {
        let bytes = file.read_line(&mut line)?;
        if let Some(prev_line) = prev_line {
            if line != prev_line {
                out_file.write_fmt(format_args!(
                    "{}",
                    format(config.count, counter, &prev_line)
                ))?;
                counter = 0;
            }
        }
        if bytes == 0 {
            break;
        }
        counter += 1;
        prev_line = Some(line.clone());
        line.clear();
    }
    Ok(())
}
