use clap::{App, Arg};
use std::{
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader, Read},
    usize,
};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    lines: usize,
    bytes: Option<usize>,
}

fn parse_positive_int(val: &str) -> MyResult<usize> {
    match val.parse() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(From::from(val)),
    }
}

#[test]
fn test_parse_positive_int() {
    let res = parse_positive_int("3");
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 3);

    let res = parse_positive_int("foo");
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().to_string(), "foo".to_string());

    let res = parse_positive_int("0");
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().to_string(), "0".to_string());
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("headr")
        .version("0.1.0")
        .author("circulene")
        .about("Rust head")
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .help("Input file(s)")
                .multiple(true)
                .default_value("-"),
        )
        .arg(
            Arg::with_name("lines")
                .short("n")
                .long("lines")
                .help("Number of lines")
                .value_name("LINES")
                .takes_value(true)
                .default_value("10"),
        )
        .arg(
            Arg::with_name("bytes")
                .short("c")
                .long("bytes")
                .help("Number of bytes")
                .value_name("BYTES")
                .takes_value(true)
                .conflicts_with("lines"),
        )
        .get_matches();

    let files = matches.values_of_lossy("files").unwrap();
    let lines = matches
        .value_of("lines")
        .map(parse_positive_int)
        .transpose()
        .map_err(|e| {
            format!(
                "error: invalid value '{}' for '--lines <LINES>': invalid digit found in string",
                e
            )
        })?
        .unwrap();
    let bytes = matches
        .value_of("bytes")
        .map(parse_positive_int)
        .transpose()
        .map_err(|e| {
            format!(
                "error: invalid value '{}' for '--bytes <BYTES>': invalid digit found in string",
                e
            )
        })?;

    Ok(Config {
        files,
        lines,
        bytes,
    })
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

pub fn run(config: Config) -> MyResult<()> {
    for (i, filename) in config.files.iter().enumerate() {
        match open(filename) {
            Err(err) => eprintln!("{}: {}", filename, err),
            Ok(mut file) => {
                // print file header
                if config.files.len() > 1 {
                    let spacer = if i > 0 { "\n" } else { "" };
                    println!("{}==> {} <==", spacer, filename);
                }

                if let Some(bytes) = config.bytes {
                    let mut handle = file.take(bytes as u64);
                    let mut buf = vec![0; bytes];
                    let size = handle.read(&mut buf)?;
                    let str = String::from_utf8_lossy(&buf[..size]);
                    print!("{}", str);
                } else {
                    let mut line = String::new();
                    for _ in 0..config.lines {
                        let size = file.read_line(&mut line)?;
                        if size == 0 {
                            break;
                        }
                        print!("{}", line);
                        line.clear();
                    }
                }
            }
        }
    }
    Ok(())
}
