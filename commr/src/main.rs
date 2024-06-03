use anyhow::{Error, Result};
use clap::{ArgAction, Parser};
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    process::exit,
};

#[derive(Parser)]
#[command(author, version, about = "Rust comm")]
pub struct Args {
    #[arg(value_name = "FILE1", help = "Input file 1")]
    file1: String,

    #[arg(value_name = "FILE2", help = "Input file 2")]
    file2: String,

    #[arg(short = '1', action = ArgAction::SetFalse, default_value = "true", help = "Supress printing of column 1")]
    show_col1: bool,

    #[arg(short = '2', action = ArgAction::SetFalse, default_value = "true", help = "Supress printing of column 2")]
    show_col2: bool,

    #[arg(short = '3', action = ArgAction::SetFalse, default_value = "true", help = "Supress printing of column 3")]
    show_col3: bool,

    #[arg(short = 'i', help = "Case-insensitive comparison of lines")]
    insensitive: bool,

    #[arg(
        short = 'd',
        long = "output-delimiter",
        default_value = "\t",
        value_name = "DELIM",
        help = "Output delimiter"
    )]
    delimiter: String,
}

fn open(filename: &str) -> Result<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => {
            let file =
                File::open(filename).map_err(|e| Error::msg(format!("{}: {}", filename, e)))?;
            Ok(Box::new(BufReader::new(file)))
        }
    }
}

pub fn run(args: &Args) -> Result<()> {
    let file1 = &args.file1;
    let file2 = &args.file2;

    if file1 == "-" && file2 == "-" {
        return Err(Error::msg("Both input files cannot be STDIN (\"-\")"));
    }

    let _file1 = open(file1)?;
    let _file2 = open(file2)?;
    println!("Opened {} and {}", file1, file2);

    Ok(())
}

fn main() {
    let args = Args::parse();
    if let Err(e) = run(&args) {
        eprintln!("{e}");
        exit(1);
    }
}
