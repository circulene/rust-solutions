use crate::Extract::*;
use anyhow::{Error, Result};
use clap::{builder::TypedValueParser, error::ErrorKind, Parser};
use regex::RegexBuilder;
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    num::NonZeroUsize,
    ops::{Range, RangeFrom, RangeTo},
    os::unix::ffi::OsStrExt,
};

#[derive(Clone)]
struct ByteParser {}

impl ByteParser {
    fn new() -> ByteParser {
        ByteParser {}
    }
}

impl TypedValueParser for ByteParser {
    type Value = u8;

    fn parse_ref(
        &self,
        _: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let bytes = value.as_bytes().to_owned();
        if bytes.len() != 1 {
            let err = clap::Error::raw(
                ErrorKind::ValueValidation,
                format!(
                    "--{} \"{}\" must be a single byte\n",
                    arg.unwrap().get_long().unwrap(),
                    value.to_string_lossy()
                ),
            );
            return Err(err);
        }
        Ok(bytes.first().unwrap().to_owned())
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum AnyRange<T> {
    From(RangeFrom<T>),
    To(RangeTo<T>),
    Range(Range<T>),
}

type PositionList = Vec<AnyRange<usize>>;

#[derive(Clone)]
struct PositionListParser {}

impl PositionListParser {
    fn new() -> Self {
        Self {}
    }
}

impl TypedValueParser for PositionListParser {
    type Value = PositionList;

    fn parse_ref(
        &self,
        _: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let value = value.to_string_lossy();
        parse_pos(&value).map_err(|message| {
            let message = format!("{} for {}", message, arg.map(|a| a.to_string()).unwrap());
            clap::Error::raw(ErrorKind::ValueValidation, format!("{message}\n"))
        })
    }
}

fn parse_index(value: &str) -> Result<usize> {
    let value_error = || Error::msg(format!("illegal list value: \"{value}\""));
    value
        .starts_with('+')
        .then(|| Err(value_error()))
        .unwrap_or_else(|| {
            value
                .parse::<NonZeroUsize>()
                .map(|val| val.get())
                .map_err(|_| value_error())
        })
}

fn parse_pos(value: &str) -> Result<PositionList> {
    let from_re = RegexBuilder::new(r"^(\d+)-$").build().unwrap();
    let to_re = RegexBuilder::new(r"^-(\d+)$").build().unwrap();
    let range_re = RegexBuilder::new(r"^(\d+)-(\d+)$").build().unwrap();
    value
        .split(',')
        .map(|val| {
            parse_index(val)
                .map(|n| AnyRange::Range(n - 1..n))
                .or_else(|err| {
                    from_re.captures(val).ok_or(err).and_then(|cap| {
                        let start = parse_index(&cap[1])?;
                        Ok(AnyRange::From(start - 1..))
                    })
                })
                .or_else(|err| {
                    to_re.captures(val).ok_or(err).and_then(|cap| {
                        let end = parse_index(&cap[1])?;
                        Ok(AnyRange::To(..end))
                    })
                })
                .or_else(|err| {
                    range_re.captures(val).ok_or(err).and_then(|cap| {
                        let start = parse_index(&cap[1])?;
                        let end = parse_index(&cap[2])?;
                        if start < end {
                            Ok(AnyRange::Range(start - 1..end))
                        } else {
                            Err(Error::msg(
                                format!("First number in range ({start}) must be lower than second number ({end})"),
                            ))
                        }
                    })
                })
        })
        .collect::<Result<_, _>>()
        .map_err(From::from)
}

#[derive(Parser, Debug)]
#[command(about = "Rust cut", version)]
struct Args {
    #[arg(value_name = "FILE")]
    files: Vec<String>,

    #[arg(
        short = 'd',
        long = "delim",
        value_name = "DELIMITER",
        default_value = "\t",
        help = "Field delimiter",
        value_parser(ByteParser::new())
    )]
    delimiter: u8,

    #[arg(
        short = 'f',
        long = "fields",
        value_name = "FIELDS",
        help = "Selected fields",
        value_parser(PositionListParser::new()),
        allow_hyphen_values(true),
        required(true),
        conflicts_with_all(["bytes", "chars"]),
    )]
    fields: Option<PositionList>,

    #[arg(
        short = 'b',
        long = "bytes",
        value_name = "BYTES",
        help = "Selected bytes",
        value_parser(PositionListParser::new()),
        allow_hyphen_values(true),
        required(true),
        conflicts_with_all(["fields", "chars"]),
    )]
    bytes: Option<PositionList>,

    #[arg(
        short = 'c',
        long = "chars",
        value_name = "CHARS",
        help = "Selected characters",
        value_parser(PositionListParser::new()),
        allow_hyphen_values(true),
        required(true),
        conflicts_with_all(["fields", "bytes"]),
    )]
    chars: Option<PositionList>,
}

impl Args {
    fn get_extract(&self) -> Option<Extract> {
        self.fields
            .as_ref()
            .map(|opt| Fields(opt.to_owned()))
            .or(self.bytes.as_ref().map(|opt| Bytes(opt.to_owned())))
            .or(self.chars.as_ref().map(|opt| Chars(opt.to_owned())))
    }
}

#[derive(Clone, Debug)]
enum Extract {
    Fields(PositionList),
    Bytes(PositionList),
    Chars(PositionList),
}

fn open(filename: &str) -> Result<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

fn extract_chars(line: &str, char_pos: &[AnyRange<usize>]) -> String {
    char_pos
        .iter()
        .flat_map(|range| {
            let chars = || line.chars();
            let range = match range.clone() {
                AnyRange::From(from) => from.start..chars().count(),
                AnyRange::To(to) => 0..to.end,
                AnyRange::Range(range) => range,
            };
            range
                .clone()
                .filter_map(|index| chars().nth(index))
                .collect::<Vec<char>>()
        })
        .collect()
}

fn extract_bytes(line: &str, char_pos: &[AnyRange<usize>]) -> String {
    let extracted_bytes = char_pos
        .iter()
        .flat_map(|range| {
            let bytes = line.as_bytes();
            let range = match range.clone() {
                AnyRange::From(from) => from.start..bytes.len(),
                AnyRange::To(to) => 0..to.end,
                AnyRange::Range(range) => range,
            };
            range
                .clone()
                .filter_map(|index| bytes.get(index).copied())
                .collect::<Vec<u8>>()
        })
        .collect::<Vec<u8>>();
    String::from_utf8_lossy(&extracted_bytes).to_string()
}

fn extract_fields(line: &str, delim: u8, char_pos: &[AnyRange<usize>]) -> String {
    char_pos
        .iter()
        .flat_map(|range| {
            let fields = || line.split(delim as char);
            let range = match range.clone() {
                AnyRange::From(from) => from.start..fields().count(),
                AnyRange::To(to) => 0..to.end,
                AnyRange::Range(range) => range,
            };
            range.filter_map(move |index| fields().nth(index))
        })
        .collect::<Vec<&str>>()
        .join(&String::from(delim as char))
}

fn main() {
    let args = Args::parse();
    for filename in &args.files {
        match open(filename) {
            Err(err) => eprintln!("{filename}: {err}"),
            Ok(reader) => {
                for line in reader.lines() {
                    let Ok(line) = line else {
                        eprintln!("{}: {}", filename, line.unwrap_err());
                        break;
                    };
                    let Some(extract) = args.get_extract() else {
                        break;
                    };
                    println!(
                        "{}",
                        match extract {
                            Bytes(pos) => {
                                extract_bytes(&line, &pos)
                            }
                            Chars(pos) => {
                                extract_chars(&line, &pos)
                            }
                            Fields(pos) => {
                                extract_fields(&line, args.delimiter, &pos)
                            }
                        }
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_parser_pos() {
        let res = parse_pos("");
        assert!(res.is_err());

        let res = parse_pos("0");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"0\"");

        let res = parse_pos("0-1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"0\"");

        let res = parse_pos("+1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"+1\"");

        let res = parse_pos("+1-2");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"+1-2\"");

        let res = parse_pos("1-+2");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"1-+2\"");

        let res = parse_pos("1,a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a\"");

        let res = parse_pos("1-a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"1-a\"");

        let res = parse_pos("a-1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a-1\"");

        let res = parse_pos("-");
        assert!(res.is_err());

        let res = parse_pos(",");
        assert!(res.is_err());

        let res = parse_pos("1,");
        assert!(res.is_err());

        let res = parse_pos("1-");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![AnyRange::From(0..)]);

        let res = parse_pos("1-1-1");
        assert!(res.is_err());

        let res = parse_pos("1-1-a");
        assert!(res.is_err());

        let res = parse_pos("1-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (1) must be lower than second number (1)"
        );

        let res = parse_pos("2-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (2) must be lower than second number (1)"
        );

        // normal cases

        let res = parse_pos("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![AnyRange::Range(0..1)]);

        let res = parse_pos("01");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![AnyRange::Range(0..1)]);

        let res = parse_pos("1,3");
        assert!(res.is_ok());
        assert_eq!(
            res.unwrap(),
            vec![AnyRange::Range(0..1), AnyRange::Range(2..3)]
        );

        let res = parse_pos("001,0003");
        assert!(res.is_ok());
        assert_eq!(
            res.unwrap(),
            vec![AnyRange::Range(0..1), AnyRange::Range(2..3)]
        );

        let res = parse_pos("1-3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![AnyRange::Range(0..3)]);

        let res = parse_pos("1,7,3-5");
        assert!(res.is_ok());
        assert_eq!(
            res.unwrap(),
            vec![
                AnyRange::Range(0..1),
                AnyRange::Range(6..7),
                AnyRange::Range(2..5)
            ]
        );

        let res = parse_pos("15,19-20");
        assert!(res.is_ok());
        assert_eq!(
            res.unwrap(),
            vec![AnyRange::Range(14..15), AnyRange::Range(18..20)]
        );

        let res = parse_pos("-3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![AnyRange::To(..3)]);

        let res = parse_pos("1,-3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![AnyRange::Range(0..1), AnyRange::To(..3)]);

        let res = parse_pos("-3,5-");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![AnyRange::To(..3), AnyRange::From(4..)]);

        let res = parse_pos("3-");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![AnyRange::From(2..)]);

        let res = parse_pos("1-3,5-");
        assert!(res.is_ok());
        assert_eq!(
            res.unwrap(),
            vec![AnyRange::Range(0..3), AnyRange::From(4..)]
        );
    }

    #[test]
    fn test_extract_chars() {
        assert_eq!(extract_chars("", &[AnyRange::Range(0..1)]), "".to_string());
        assert_eq!(
            extract_chars("ábc", &[AnyRange::Range(0..1)]),
            "á".to_string()
        );
        assert_eq!(
            extract_chars("ábc", &[AnyRange::Range(0..1), AnyRange::Range(2..3)]),
            "ác".to_string()
        );
        assert_eq!(
            extract_chars("ábc", &[AnyRange::Range(0..3)]),
            "ábc".to_string()
        );
        assert_eq!(
            extract_chars("ábc", &[AnyRange::Range(2..3), AnyRange::Range(1..2)]),
            "cb".to_string()
        );
        assert_eq!(
            extract_chars(
                "ábc",
                &[
                    AnyRange::Range(0..1),
                    AnyRange::Range(1..2),
                    AnyRange::Range(4..5)
                ]
            ),
            "áb".to_string()
        );
    }

    #[test]
    fn test_extract_bytes() {
        assert_eq!(
            extract_bytes("ábc", &[AnyRange::Range(0..1)]),
            "�".to_string()
        );
        assert_eq!(
            extract_bytes("ábc", &[AnyRange::Range(0..2)]),
            "á".to_string()
        );
        assert_eq!(
            extract_bytes("ábc", &[AnyRange::Range(0..3)]),
            "áb".to_string()
        );
        assert_eq!(
            extract_bytes("ábc", &[AnyRange::Range(0..4)]),
            "ábc".to_string()
        );
        assert_eq!(
            extract_bytes("ábc", &[AnyRange::Range(3..4), AnyRange::Range(2..3)]),
            "cb".to_string()
        );
        assert_eq!(
            extract_bytes("ábc", &[AnyRange::Range(0..2), AnyRange::Range(5..6)]),
            "á".to_string()
        );
    }
}
