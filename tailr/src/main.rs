use crate::TakeValue::*;
use anyhow::{Error, Result};
use clap::{builder::TypedValueParser, command, Arg, Command, Parser};
use once_cell::sync::OnceCell;
use regex::Regex;

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

fn main() {
    let args = Args::parse();
    dbg!(args);
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
}
