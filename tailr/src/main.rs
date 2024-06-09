use crate::TakeValue::*;
use clap::{builder::TypedValueParser, command, Arg, Command, Parser};
use regex::Regex;

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
        let err = |val| {
            let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation);
            if let Some(arg) = arg {
                err.insert(
                    clap::error::ContextKind::InvalidArg,
                    clap::error::ContextValue::String(arg.to_string()),
                );
            }
            err.insert(
                clap::error::ContextKind::InvalidValue,
                clap::error::ContextValue::String(val),
            );
            Err(err)
        };
        let value = value.to_string_lossy();
        let re = Regex::new(r"^([+-]?)(\d+)$").expect("Inalid regex");
        let caps = re.captures(&value);
        match caps {
            Some(caps) => {
                let sign = caps.get(1).expect("Invalid regex").as_str();
                let num = caps.get(2).expect("Invalid regex").as_str();
                let num = num.parse::<i64>().expect("Invalid number");
                if sign == "+" {
                    if num == 0 {
                        Ok(PlusZero)
                    } else {
                        Ok(TakeNum(num))
                    }
                } else {
                    Ok(TakeNum(-num))
                }
            }
            None => err(value.to_string()),
        }
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
        default_value = "10",
        value_parser(TakeValueParser::new())
    )]
    lines: TakeValue,

    /// Number of bytes
    #[arg(
        short = 'c',
        long = "bytes",
        value_name = "BYTES",
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
