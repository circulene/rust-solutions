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

    let lines1 = open(file1)?
        .lines()
        .map(|x| x.unwrap())
        .collect::<Vec<String>>();
    let lines2 = open(file2)?
        .lines()
        .map(|x| x.unwrap())
        .collect::<Vec<String>>();
    let mut common_ids: Vec<[usize; 2]> = vec![];

    for (i1, line1) in lines1.iter().enumerate() {
        for (i2, line2) in lines2.iter().enumerate() {
            let matched = if args.insensitive {
                line1.eq_ignore_ascii_case(line2)
            } else {
                line1.eq(line2)
            };
            if matched {
                common_ids.push([i1, i2]);
            }
        }
    }

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

    let mut lines1 = lines1.iter();
    let mut lines2 = lines2.iter();
    let mut common_ids_iter = common_ids.iter();
    let mut last_common_id = [0, 0];
    loop {
        let common_id = common_ids_iter.next();
        match common_id {
            Some(common_id) => {
                let file2_range = last_common_id[1]..common_id[1];
                for _ in file2_range {
                    let line = lines2.next().unwrap();
                    print2(line);
                }

                let file1_range = last_common_id[0]..common_id[0];
                for _ in file1_range {
                    let line = lines1.next().unwrap();
                    print1(line);
                }

                let line = lines1.next().unwrap();
                let _ = lines2.next();
                print3(line);

                last_common_id = [common_id[0] + 1, common_id[1] + 1]
            }
            None => {
                for line1 in lines1.by_ref() {
                    print1(line1);
                }
                for line2 in lines2.by_ref() {
                    print2(line2);
                }
                break;
            }
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
