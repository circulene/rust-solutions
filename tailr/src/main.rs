use crate::TakeValue::*;
use anyhow::{Error, Result};
use clap::{builder::TypedValueParser, command, Arg, Command, Parser};
use once_cell::sync::OnceCell;
use regex::Regex;
use std::{
    cmp::max,
    fs::File,
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
};

static NUM_RE: OnceCell<Regex> = OnceCell::new();

#[derive(PartialEq, Clone, Debug)]
enum TakeValue {
    PlusZero,
    TakeNum(i64),
}

#[derive(Clone)]
struct TakeValueParser {}

impl TakeValueParser {
    fn new() -> Self {
        Self {}
    }
}

impl TypedValueParser for TakeValueParser {
    type Value = TakeValue;

    fn parse_ref(
        &self,
        _: &Command,
        arg: Option<&Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        parse_num(&value.to_string_lossy()).map_err(|e| {
            let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation);
            if let Some(arg) = arg {
                err.insert(
                    clap::error::ContextKind::InvalidArg,
                    clap::error::ContextValue::String(arg.to_string()),
                );
            }
            err.insert(
                clap::error::ContextKind::InvalidValue,
                clap::error::ContextValue::String(e.to_string()),
            );
            err
        })
    }
}

fn parse_num(value: &str) -> Result<TakeValue> {
    let re = NUM_RE.get_or_init(|| Regex::new(r"^([+-]?)\d+$").expect("Inalid regex"));
    let caps = re.captures(value);
    match caps {
        Some(caps) => {
            let sign = caps.get(1).expect("Invalid regex").as_str();
            let num = value.parse::<i64>().expect("Invalid number");
            if sign == "+" {
                if num == 0 {
                    Ok(PlusZero)
                } else {
                    Ok(TakeNum(num))
                }
            } else if sign == "-" {
                Ok(TakeNum(num))
            } else {
                Ok(TakeNum(-num))
            }
        }
        None => Err(Error::msg(value.to_string())),
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input file(s)
    #[arg(value_name = "FILE", required = true)]
    files: Vec<String>,

    /// Number of lines
    #[arg(
        short = 'n',
        long = "lines",
        value_name = "LINES",
        allow_hyphen_values = true,
        default_value = "10",
        conflicts_with = "bytes",
        value_parser(TakeValueParser::new())
    )]
    lines: TakeValue,

    /// Number of bytes
    #[arg(
        short = 'c',
        long = "bytes",
        value_name = "BYTES",
        allow_hyphen_values = true,
        conflicts_with = "lines",
        value_parser(TakeValueParser::new())
    )]
    bytes: Option<TakeValue>,

    /// Supress headers
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,
}

fn open_file(filename: &str) -> Result<File> {
    File::open(filename).map_err(|e| Error::msg(format!("{}: {}", filename, e)))
}

fn open_bufread(filename: &str) -> Result<Box<dyn BufRead>> {
    let file = open_file(filename)?;
    Ok(Box::new(BufReader::new(file)))
}

fn count_lines_bytes(filename: &str) -> Result<(i64, i64)> {
    let lines: i64 = open_bufread(filename)?.lines().count() as i64;
    let mut buf = String::new();
    let mut bytes: i64 = 0;
    let mut file = open_bufread(filename)?;
    loop {
        let read_bytes = file.read_line(&mut buf)?;
        if read_bytes == 0 {
            break;
        }
        bytes += read_bytes as i64;
        buf.clear();
    }
    Ok((lines, bytes))
}

fn get_start_index(take_val: &TakeValue, total: i64) -> Option<i64> {
    match take_val {
        TakeNum(num) => {
            let num = *num;
            if num == 0 || total == 0 || num > total {
                None
            } else if num < 0 {
                Some(max(total + num, 0))
            } else {
                Some(num - 1)
            }
        }
        PlusZero => {
            if total != 0 {
                Some(0)
            } else {
                None
            }
        }
    }
}

fn print_header(i: usize, filename: &str) {
    if i > 0 {
        println!();
    }
    println!("==> {} <==", filename);
}

fn print_lines(mut file: impl BufRead, num_lines: &TakeValue, total_lines: i64) -> Result<()> {
    if let Some(start) = get_start_index(num_lines, total_lines) {
        let mut line = String::new();
        for i in 0..total_lines {
            file.read_line(&mut line)?;
            if i >= start {
                print!("{}", line);
            }
            line.clear();
        }
    }
    Ok(())
}

fn print_bytes<T>(mut file: T, num_bytes: &TakeValue, total_bytes: i64) -> Result<()>
where
    T: Read + Seek,
{
    if let Some(start) = get_start_index(num_bytes, total_bytes) {
        file.seek(SeekFrom::Start(start as u64))?;
        let mut buf = vec![0; (total_bytes - start) as usize];
        file.read_exact(&mut buf)?;
        print!("{}", String::from_utf8_lossy(&buf));
    }
    Ok(())
}

fn run(args: Args) -> Result<()> {
    for (i, filename) in args.files.iter().enumerate() {
        let (total_lines, total_bytes) = count_lines_bytes(filename)?;
        if args.files.len() > 1 && !args.quiet {
            print_header(i, filename);
        }
        if let Some(bytes) = &args.bytes {
            let file = open_file(filename)?;
            print_bytes(file, bytes, total_bytes)?;
        } else {
            let file = open_bufread(filename)?;
            print_lines(file, &args.lines, total_lines)?;
        }
    }
    Ok(())
}

fn main() {
    let args = Args::parse();
    if let Err(err) = run(args) {
        eprintln!("{}", err);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_num() {
        let res = parse_num("3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(-3));

        let res = parse_num("+3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(3));

        let res = parse_num("-3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(-3));

        let res = parse_num("0");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(0));

        let res = parse_num("+0");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), PlusZero);

        let res = parse_num(&i64::MAX.to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN + 1));

        let res = parse_num(&(i64::MIN + 1).to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN + 1));

        let res = parse_num(&format!("+{}", i64::MAX));
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MAX));

        let res = parse_num(&i64::MIN.to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN));

        let res = parse_num("3.14");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "3.14");

        let res = parse_num("foo");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "foo");
    }

    #[test]
    fn test_count_lines_bytes() {
        let res = count_lines_bytes("tests/inputs/one.txt");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), (1, 24));

        let res = count_lines_bytes("tests/inputs/twelve.txt");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), (12, 63));
    }

    #[test]
    fn test_get_start_index() {
        assert_eq!(get_start_index(&PlusZero, 0), None);

        assert_eq!(get_start_index(&PlusZero, 1), Some(0));

        assert_eq!(get_start_index(&TakeNum(0), 1), None);

        assert_eq!(get_start_index(&TakeNum(1), 0), None);

        assert_eq!(get_start_index(&TakeNum(2), 1), None);

        assert_eq!(get_start_index(&TakeNum(1), 10), Some(0));
        assert_eq!(get_start_index(&TakeNum(2), 10), Some(1));
        assert_eq!(get_start_index(&TakeNum(3), 10), Some(2));

        assert_eq!(get_start_index(&TakeNum(-1), 10), Some(9));
        assert_eq!(get_start_index(&TakeNum(-2), 10), Some(8));
        assert_eq!(get_start_index(&TakeNum(-3), 10), Some(7));

        assert_eq!(get_start_index(&TakeNum(-20), 10), Some(0));
    }
}
