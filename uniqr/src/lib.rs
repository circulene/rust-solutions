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

fn print_format(
    out_file: &mut Box<dyn Write>,
    show_count: bool,
    counter: usize,
    line: &str,
) -> Result<()> {
    if show_count {
        out_file.write_fmt(format_args!("{counter:>4} {line}"))?
    } else {
        out_file.write_fmt(format_args!("{line}"))?
    }
    Ok(())
}

pub fn run(config: Config) -> Result<()> {
    let mut file =
        open(&config.in_file).map_err(|e| Error::msg(format!("{}: {}", &config.in_file, e)))?;
    let mut out_file: Box<dyn Write> = match &config.out_file {
        Some(out_name) => Box::new(File::create(out_name)?),
        _ => Box::new(io::stdout()),
    };
    let mut line = String::new();
    let mut prev_line = String::new();
    let mut counter: usize = 0;
    loop {
        let bytes = file.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }
        if counter > 0 {
            if line.trim_end() != prev_line.trim_end() {
                print_format(&mut out_file, config.count, counter, &prev_line)?;
                counter = 0;
                prev_line = line.clone();
            }
        } else {
            prev_line = line.clone();
        }
        counter += 1;
        line.clear();
    }
    if counter > 0 {
        print_format(&mut out_file, config.count, counter, &prev_line)?;
    }
    Ok(())
}
