use crate::Extract::*;
use clap::{
    builder::TypedValueParser,
    error::{ContextKind, ContextValue, ErrorKind},
    value_parser, Parser,
};
use regex::{Captures, RegexBuilder};
use std::{error::Error, ops::Range, os::unix::ffi::OsStrExt};

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
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let bytes = value.as_bytes().to_owned();
        bytes.first().map(|x| x.to_owned()).ok_or_else(|| {
            let message = format!(
                "invalid byte '{}' for {}",
                value.to_string_lossy(),
                arg.map(|a| a.to_string()).unwrap()
            );
            clap::Error::raw(ErrorKind::ValueValidation, format!("{message}\n"))
        })
    }
}

type PositionList = Vec<Range<usize>>;

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
        cmd: &clap::Command,
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

fn parse_pos(value: &str) -> Result<PositionList, String> {
    let re = RegexBuilder::new(r"^(\d+)(-(\d+))?$").build().unwrap();
    let captured = |cap: &Captures, i| cap.get(i).map(|m| m.as_str().parse::<usize>().unwrap());
    let mut result = Vec::new();

    for range_str in value.split(',') {
        let Some(cap) = re.captures(range_str) else {
            return Err(format!("illegal list value: \"{range_str}\""));
        };
        let range = match (captured(&cap, 1), captured(&cap, 3)) {
            (Some(start), Some(end)) => {
                if start < end {
                    if start > 0 {
                        Ok(start - 1..end)
                    } else {
                        Err(format!("illegal list value: \"{start}\""))
                    }
                } else {
                    Err(format!(
                        "First number in range ({start}) must be lower than second number ({end})"
                    ))
                }
            }
            (Some(end), None) => {
                if end > 0 {
                    Ok(end - 1..end)
                } else {
                    Err(format!("illegal list value: \"{end}\""))
                }
            }
            (_, _) => Err(format!("illegal list value: \"{range_str}\"")),
        };
        result.push(range?);
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parser() {
        let res = parse_pos("");
        assert!(res.is_err());

        let res = parse_pos("0");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "illegal list value: \"0\"");

        let res = parse_pos("0-1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "illegal list value: \"0\"");

        let res = parse_pos("+1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "illegal list value: \"+1\"");

        let res = parse_pos("+1-2");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "illegal list value: \"+1-2\"");

        let res = parse_pos("1-+2");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "illegal list value: \"1-+2\"");

        let res = parse_pos("1,a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "illegal list value: \"a\"");

        let res = parse_pos("1-a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "illegal list value: \"1-a\"");

        let res = parse_pos("a-1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "illegal list value: \"a-1\"");

        let res = parse_pos("-");
        assert!(res.is_err());

        let res = parse_pos(",");
        assert!(res.is_err());

        let res = parse_pos("1,");
        assert!(res.is_err());

        let res = parse_pos("1-");
        assert!(res.is_err());

        let res = parse_pos("1-1-1");
        assert!(res.is_err());

        let res = parse_pos("1-1-a");
        assert!(res.is_err());

        let res = parse_pos("1-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            "First number in range (1) must be lower than second number (1)"
        );

        let res = parse_pos("2-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            "First number in range (2) must be lower than second number (1)"
        );

        // normal cases

        let res = parse_pos("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("01");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("1,3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("001,0003");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("1-3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_pos("1,7,3-5");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 6..7, 2..5]);

        let res = parse_pos("15,19-20");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![14..15, 18..20]);
    }
}

#[derive(Parser, Debug)]
#[command(about = "Rust cut", version)]
struct Args {
    #[arg(value_name = "FILE")]
    files: Vec<String>,

    #[arg(
        short = 'd',
        long = "delimiter",
        value_name = "DELIMITER",
        default_value = " ",
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
        conflicts_with_all(["bytes", "chars"]),
    )]
    fields: Option<PositionList>,

    #[arg(
        short = 'b',
        long = "bytes",
        value_name = "BYTES",
        help = "Selected bytes",
        value_parser(PositionListParser::new()),
        conflicts_with_all(["fields", "chars"]),
    )]
    bytes: Option<PositionList>,

    #[arg(
        short = 'c',
        long = "chars",
        value_name = "CHARS",
        help = "Selected characters",
        value_parser(PositionListParser::new()),
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

fn main() {
    let args = Args::parse();
    dbg!(args);
}
