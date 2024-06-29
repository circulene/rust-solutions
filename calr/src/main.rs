use std::{iter::zip, process::exit, str::FromStr};

use ansi_term::Style;
use anyhow::{Error, Result};
use chrono::{Datelike, Days, Local, Months, NaiveDate, Weekday};
use clap::Parser;

const VALID_MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

#[derive(Parser)]
pub struct Args {
    /// Year (1-9999)
    #[arg(value_name = "YEAR", value_parser(clap::value_parser!(i32).range(1..=9999)))]
    year: Option<i32>,

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
                if valid_name
                    .to_lowercase()
                    .starts_with::<&str>(month.to_lowercase().as_ref())
                {
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

fn format_month(year: i32, month: u32, print_year: bool, today: NaiveDate) -> Vec<String> {
    let width = 20;
    let last_space = "  ";
    let mut format_month = vec![];
    format_month.push(format!(
        "{:^width$}  ",
        format!(
            "{}{}",
            VALID_MONTH_NAMES[month as usize - 1],
            if print_year {
                format!(" {}", year)
            } else {
                "".to_string()
            }
        )
    ));
    format_month.push(format!("{:<width$}{}", "Su Mo Tu We Th Fr Sa", last_space));

    let first_day_in_month = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let num_weeks_in_month = 6;
    let emphasize = |day: String| Style::new().reverse().paint(day).to_string();
    for sunday in first_day_in_month
        .week(Weekday::Sun)
        .first_day()
        .iter_weeks()
        .take(num_weeks_in_month)
    {
        let mut format_days_in_week = vec![];
        for weekday in sunday.iter_days().take(7) {
            if weekday.month() == month {
                let format_day = format!("{:>2}", weekday.day());
                format_days_in_week.push(if weekday == today {
                    emphasize(format_day)
                } else {
                    format_day
                });
            } else {
                format_days_in_week.push("  ".to_owned());
            }
        }
        format_month.push(format!("{}{}", format_days_in_week.join(" "), last_space));
    }
    format_month
}

#[allow(dead_code)]
fn last_day_in_month(year: i32, month: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, 1).unwrap() + Months::new(1) - Days::new(1)
}

fn show_whole_year(year: i32, today: NaiveDate) {
    println!("{:>32}", year);
    for quarter in 1..=4 {
        let month_in_quarter = (quarter - 1) * 3 + 1;
        let m1 = format_month(year, month_in_quarter, false, today);
        let m2 = format_month(year, month_in_quarter + 1, false, today);
        let m3 = format_month(year, month_in_quarter + 2, false, today);
        for ((s1, s2), s3) in zip(zip(m1, m2), m3) {
            println!("{}{}{}", s1, s2, s3);
        }
        if quarter < 4 {
            println!();
        }
    }
}

fn run(args: &Args) -> Result<()> {
    let today = Local::now().date_naive();
    if args.show_current_year {
        show_whole_year(today.year(), today);
    } else {
        let year = args.year;
        let month = args
            .month
            .as_ref()
            .map(|month| parse_month(month))
            .transpose()?;
        match (year, month) {
            (Some(year), None) => show_whole_year(year, today),
            _ => {
                let year = year.unwrap_or(today.year());
                let month = month.unwrap_or(today.month());
                for s in format_month(year, month, true, today) {
                    println!("{}", s);
                }
            }
        }
    }
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

    #[test]
    fn test_format_month() {
        let today = NaiveDate::from_ymd_opt(0, 1, 1).unwrap();
        let leap_february = vec![
            "   February 2020      ",
            "Su Mo Tu We Th Fr Sa  ",
            "                   1  ",
            " 2  3  4  5  6  7  8  ",
            " 9 10 11 12 13 14 15  ",
            "16 17 18 19 20 21 22  ",
            "23 24 25 26 27 28 29  ",
            "                      ",
        ];
        assert_eq!(format_month(2020, 2, true, today), leap_february);

        let may = vec![
            "        May           ",
            "Su Mo Tu We Th Fr Sa  ",
            "                1  2  ",
            " 3  4  5  6  7  8  9  ",
            "10 11 12 13 14 15 16  ",
            "17 18 19 20 21 22 23  ",
            "24 25 26 27 28 29 30  ",
            "31                    ",
        ];
        assert_eq!(format_month(2020, 5, false, today), may);

        let april_hl = vec![
            "     April 2021       ",
            "Su Mo Tu We Th Fr Sa  ",
            "             1  2  3  ",
            " 4  5  6 \u{1b}[7m 7\u{1b}[0m  8  9 10  ",
            "11 12 13 14 15 16 17  ",
            "18 19 20 21 22 23 24  ",
            "25 26 27 28 29 30     ",
            "                      ",
        ];
        let today = NaiveDate::from_ymd_opt(2021, 4, 7).unwrap();
        assert_eq!(format_month(2021, 4, true, today), april_hl);
    }

    #[test]
    fn test_last_day_in_month() {
        assert_eq!(
            last_day_in_month(2020, 1),
            NaiveDate::from_ymd_opt(2020, 1, 31).unwrap()
        );
        assert_eq!(
            last_day_in_month(2020, 2),
            NaiveDate::from_ymd_opt(2020, 2, 29).unwrap()
        );
        assert_eq!(
            last_day_in_month(2020, 4),
            NaiveDate::from_ymd_opt(2020, 4, 30).unwrap()
        )
    }
}
