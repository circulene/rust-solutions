use std::{process::exit, str::FromStr};

use anyhow::{Error, Result};
use chrono::Local;
use clap::Parser;

const VALID_MONTH_NAMES: [&str; 12] = [
    "january",
    "feburary",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

#[derive(Parser)]
pub struct Args {
    /// Year (1-9999)
    #[arg(value_name = "YEAR")]
    year: Option<String>,

    /// Month name or number (1-12)
    #[arg(short = 'm', value_name = "MONTH", conflicts_with("show_current_year"))]
    month: Option<String>,

    /// Show whole current year
    #[arg(short = 'y', long = "year", conflicts_with_all(["year", "month"]) )]
    show_current_year: bool,
}

fn parse_int<T: FromStr>(val: &str) -> Result<T> {
    val.parse::<T>()
        .map_err(|_| Error::msg(format!("Invalid integer \"{}\"", val)))
}

fn parse_year(year: &str) -> Result<i32> {
    let year_range = 1..=9999;
    parse_int::<i32>(year).and_then(|v| {
        if year_range.contains(&v) {
            Ok(v)
        } else {
            Err(Error::msg(format!(
                "year \"{}\" not in the range {} through {}",
                year,
                year_range.start(),
                year_range.end()
            )))
        }
    })
}

fn parse_month(month: &str) -> Result<u32> {
    let month_range = 1..=12;
    match parse_int::<u32>(month) {
        Ok(month) => {
            if month_range.contains(&month) {
                Ok(month)
            } else {
                Err(Error::msg(format!(
                    "month \"{}\" not in the range {} through {}",
                    month,
                    month_range.start(),
                    month_range.end()
                )))
            }
        }
        _ => {
            let mut candidate = None;
            for (i, valid_name) in VALID_MONTH_NAMES.iter().enumerate() {
                if valid_name.starts_with::<&str>(month.to_lowercase().as_ref()) {
                    if candidate.is_some() {
                        candidate = None;
                        break;
                    }
                    candidate = Some(i as u32 + 1);
                }
            }
            candidate.ok_or(Error::msg(format!("Invalid month \"{}\"", month)))
        }
    }
}

fn run(args: &Args) -> Result<()> {
    let today = Local::now();
    let year = args
        .year
        .as_ref()
        .map(|year| parse_year(year))
        .transpose()?;
    let month = args
        .month
        .as_ref()
        .map(|month| parse_month(month))
        .transpose()?;
    Ok(())
}

fn main() {
    let args = Args::parse();
    if let Err(e) = run(&args) {
        eprintln!("{}", e);
        exit(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_int() {
        let res = parse_int::<usize>("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1usize);

        let res = parse_int::<i32>("-1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), -1i32);

        let res = parse_int::<i64>("foo");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "Invalid integer \"foo\"");
    }

    #[test]
    fn test_parse_year() {
        let res = parse_year("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1i32);

        let res = parse_year("9999");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 9999i32);

        let res = parse_year("0");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "year \"0\" not in the range 1 through 9999"
        );

        let res = parse_year("10000");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "year \"10000\" not in the range 1 through 9999"
        );

        let res = parse_year("foo");
        assert!(res.is_err());
    }

    #[test]
    fn test_parse_month() {
        let res = parse_month("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1u32);

        let res = parse_month("12");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 12u32);

        let res = parse_month("jan");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1u32);

        let res = parse_month("0");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "month \"0\" not in the range 1 through 12"
        );

        let res = parse_month("13");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "month \"13\" not in the range 1 through 12"
        );

        let res = parse_month("foo");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "Invalid month \"foo\"");
    }
}
