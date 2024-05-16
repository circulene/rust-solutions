use crate::Extract::*;
use clap::{
    builder::TypedValueParser,
    error::{ContextKind, ContextValue, ErrorKind},
    Parser,
};
use regex::{Captures, RegexBuilder};
use std::ops::Range;

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
            let mut err = clap::Error::raw(ErrorKind::InvalidValue, message).with_cmd(cmd);
            if let Some(arg) = arg {
                err.insert(
                    ContextKind::InvalidArg,
                    ContextValue::String(arg.to_string()),
                );
            }
            err
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
                if start == 0 {
                    Err(format!("illegal list value: \"{start}\""))
                } else if start < end {
                    Ok(start - 1..end)
                } else {
                    Err(format!(
                        "First number in range ({start}) must be lower than second number ({end})"
                    ))
                }
            }
            (Some(end), None) => {
                if end == 0 {
                    Err(format!("illegal list value: \"{end}\""))
                } else {
                    Ok(end - 1..end)
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
        help = "Field delimiter"
    )]
    delimiter: u8,

    #[arg(
        short = 'f',
        long = "fields",
        value_name = "FIELDS",
        help = "Selected fields",
        value_parser = PositionListParser::new()
    )]
    fields: Option<PositionList>,

    #[arg(
        short = 'b',
        long = "bytes",
        value_name = "BYTES",
        help = "Selected bytes",
        value_parser = PositionListParser::new()
    )]
    bytes: Option<PositionList>,

    #[arg(
        short = 'c',
        long = "chars",
        value_name = "CHARS",
        help = "Selected characters",
        value_parser = PositionListParser::new()
    )]
    chars: Option<PositionList>,
}

impl Args {
    fn get_extract(&self) -> Option<Extract> {
        if let Some(fields) = &self.fields {
            Some(Fields(fields.to_vec()))
        } else if let Some(bytes) = &self.bytes {
            Some(Bytes(bytes.to_vec()))
        } else if let Some(chars) = &self.chars {
            Some(Chars(chars.to_vec()))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
enum Extract {
    Fields(PositionList),
    Bytes(PositionList),
    Chars(PositionList),
}

struct Config {
    files: Vec<String>,
    delimiter: u8,
    extract: Option<Extract>,
}

fn main() {
    let args = Args::parse();
    let extract = args.get_extract();
    let config = Config {
        files: args.files,
        delimiter: args.delimiter,
        extract: None,
    };
}
