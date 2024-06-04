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

    let mut file1_line_nums: Vec<usize> = vec![];
    let mut file2_line_nums: Vec<usize> = vec![];
    for (i1, line1) in open(file1)?.lines().enumerate() {
        let line1 = &line1?;
        for (i2, line2) in open(file2)?.lines().enumerate() {
            let line2 = &line2?;
            if *line1 == *line2 {
                file1_line_nums.push(i1 + 1);
                file2_line_nums.push(i2 + 1);
            }
        }
    }

    let delim = &args.delimiter;
    let mut lines1 = open(file1)?.lines();
    let mut lines2 = open(file2)?.lines();
    let mut i1 = file1_line_nums.iter();
    let mut i2 = file2_line_nums.iter();
    let mut last_i1 = 0;
    let mut last_i2 = 0;
    loop {
        let i1 = i1.next();
        let i2 = i2.next();
        if i1.is_none() && i2.is_none() {
            for line1 in lines1.by_ref() {
                println!("{}", line1?);
            }
            for line2 in lines2.by_ref() {
                println!("{}{}", delim, line2?);
            }
            break;
        }
        if let Some(i1) = i1 {
            for _ in last_i1..*i1 - 1 {
                let line = lines1.next().transpose()?.unwrap();
                println!("{}", line);
            }
            last_i1 = *i1;
        }
        if let Some(i2) = i2 {
            for _ in last_i2..*i2 - 1 {
                let line = lines2.next().transpose()?.unwrap();
                println!("{}{}", delim, line);
            }
            last_i2 = *i2;
        }
        if let Some(i1) = i1 {
            let line = lines1.next().transpose()?.unwrap();
            lines2.next();
            println!("{}{}{}", delim, delim, line);
        }
    }

    Ok(())
}

fn main() {
    let args = Args::parse();
    if let Err(e) = run(&args) {
        eprintln!("{e}");
        exit(1);
    }
}
