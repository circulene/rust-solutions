use anyhow::{Error, Result};
use clap::{ArgAction, Parser};
use std::{
    cmp::Ordering::{Equal, Greater, Less},
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

    let compare = |s1: &str, s2: &str| {
        if args.insensitive {
            s1.to_lowercase().cmp(&s2.to_lowercase())
        } else {
            s1.cmp(s2)
        }
    };

    let print1 = |s: &str| {
        if args.show_col1 {
            println!("{}", s);
        }
    };

    let print2 = |s: &str| {
        if args.show_col2 {
            if args.show_col1 {
                print!("{}", args.delimiter);
            }
            println!("{}", s);
        }
    };

    let print3 = |s: &str| {
        if args.show_col3 {
            if args.show_col1 {
                print!("{}", args.delimiter);
            }
            if args.show_col2 {
                print!("{}", args.delimiter);
            }
            println!("{}", s);
        }
    };

    let mut lines1 = open(file1)?.lines().map_while(Result::ok);
    let mut lines2 = open(file2)?.lines().map_while(Result::ok);

    let mut line1 = lines1.next();
    let mut line2 = lines2.next();
    loop {
        match (&line1, &line2) {
            (Some(s1), Some(s2)) => match compare(s1, s2) {
                Less => {
                    print1(s1);
                    line1 = lines1.next();
                }
                Greater => {
                    print2(s2);
                    line2 = lines2.next();
                }
                Equal => {
                    print3(s1);
                    line1 = lines1.next();
                    line2 = lines2.next();
                }
            },
            (Some(s1), None) => {
                print1(s1);
                line1 = lines1.next();
            }
            (None, Some(s2)) => {
                print2(s2);
                line2 = lines2.next();
            }
            (None, None) => break,
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
