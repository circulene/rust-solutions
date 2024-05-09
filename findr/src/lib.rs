use anyhow::Result;
use clap::{
    builder::{PossibleValue, TypedValueParser},
    error::{ContextKind, ContextValue, ErrorKind},
    Parser, ValueEnum,
};
use regex::Regex;
use std::{fmt::Debug, os::unix::fs::MetadataExt};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Eq, PartialEq, Clone)]
enum EntryType {
    Dir,
    File,
    Link,
}

impl ValueEnum for EntryType {
    fn value_variants<'a>() -> &'a [Self] {
        &[EntryType::Dir, EntryType::File, EntryType::Link]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            EntryType::Dir => PossibleValue::new("d").into(),
            EntryType::File => PossibleValue::new("f").into(),
            EntryType::Link => PossibleValue::new("l").into(),
        }
    }
}

#[derive(Debug, Clone)]
enum CmpFlag {
    Plus,
    Minus,
    None,
}

#[derive(Debug, Clone)]
struct SizeType {
    size: u64,
    blksize: u64,
    cmp_flag: CmpFlag,
}

#[derive(Clone)]
struct SizeTypeParser {}

impl SizeTypeParser {
    fn new() -> Self {
        Self {}
    }
}

impl TypedValueParser for SizeTypeParser {
    type Value = SizeType;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        TypedValueParser::parse(self, cmd, arg, value.to_owned())
    }

    fn parse(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: std::ffi::OsString,
    ) -> Result<Self::Value, clap::Error> {
        let value = value
            .into_string()
            .map_err(|_e| clap::Error::new(ErrorKind::InvalidUtf8).with_cmd(cmd))?;
        let validation_error = |suggest: Option<String>| {
            let mut err = clap::Error::new(ErrorKind::ValueValidation).with_cmd(cmd);
            if let Some(arg) = arg {
                err.insert(
                    ContextKind::InvalidArg,
                    ContextValue::String(arg.to_string()),
                );
            }
            err.insert(
                ContextKind::InvalidValue,
                ContextValue::String(value.to_string()),
            );
            if let Some(suggest) = suggest {
                err.insert(ContextKind::SuggestedValue, ContextValue::String(suggest));
            }
            err
        };
        let pattern = Regex::new(r"(?<flag>.*?)(?<size>[0-9]+)(?<unit>.*)").unwrap();
        if let Some(cap) = pattern.captures(&value) {
            let cmp_flag = cap
                .name("flag")
                .map(|m| {
                    let flag = m.as_str();
                    match flag {
                        "+" => Ok(CmpFlag::Plus),
                        "-" => Ok(CmpFlag::Minus),
                        "" => Ok(CmpFlag::None),
                        _ => Err({
                            validation_error(Some(format!("Flag '{flag}' is invalid. Possible values are any of '+', '-' or ''.")))
                        }),
                    }
                })
                .transpose()?
                .unwrap();
            let size = cap
                .name("size")
                .map(|m| m.as_str().parse::<u64>().unwrap())
                .unwrap();
            let unit = cap.name("unit").map(|m| m.as_str()).unwrap();
            let blksize: u64 = match unit {
                "b" => Ok(512),
                "c" => Ok(1),
                "k" => Ok(1024),
                "M" => Ok(1024 * 1024),
                "G" => Ok(1024 * 1024 * 1024),
                "T" => Ok(1024 * 1024 * 1024 * 1024),
                "" => Ok(512),
                _ => Err(validation_error(Some(format!(
                    "Unit '{unit}' is invalid. Possible values are any of 'b', 'c', 'k', 'M', 'G', 'T' or ''."
                )))),
            }?;
            Ok(Self::Value {
                cmp_flag,
                size,
                blksize,
            })
        } else {
            Err(validation_error(None))
        }
    }
}

#[derive(Debug, Parser)]
#[command(about = "Rust find", version)]
pub struct Config {
    /// Search paths
    #[arg(value_name = "PATH", default_value = ".")]
    paths: Vec<String>,

    /// Name
    #[arg(short = 'n', long = "name", value_name = "NAME", num_args(0..))]
    names: Vec<Regex>,

    /// Entry type
    #[arg(short = 't', long = "type", value_name = "TYPE", num_args(0..), value_enum)]
    entry_types: Vec<EntryType>,

    /// Minimum depth
    #[arg(long = "mindepth")]
    min_depth: Option<usize>,

    /// Maximum depth
    #[arg(long = "maxdepth")]
    max_depth: Option<usize>,

    /// File size. Format is similar to find, e.g. [+-]?[0-9]+[ckMGT]?
    #[arg(
        long = "size",
        allow_hyphen_values = true,
        value_parser(SizeTypeParser::new())
    )]
    size_type: Option<SizeType>,
}

pub fn get_args() -> Result<Config> {
    let config = Config::try_parse()?;
    Ok(config)
}

pub fn run(config: Config) -> Result<()> {
    let walk_dir = |path: &String| {
        let mut walk_dir = WalkDir::new(path);
        if let Some(depth) = config.min_depth {
            walk_dir = walk_dir.min_depth(depth);
        }
        if let Some(depth) = config.max_depth {
            walk_dir = walk_dir.max_depth(depth);
        }
        walk_dir
    };
    let name_filter = |entry: &DirEntry| {
        config.names.is_empty()
            || config
                .names
                .iter()
                .any(|regex| regex.is_match(&entry.file_name().to_string_lossy()))
    };
    let entry_type_filter = |entry: &DirEntry| {
        let file_type = entry.file_type();
        config.entry_types.is_empty()
            || config
                .entry_types
                .iter()
                .any(|entry_type| match entry_type {
                    EntryType::Dir => file_type.is_dir(),
                    EntryType::File => file_type.is_file(),
                    EntryType::Link => file_type.is_symlink(),
                })
    };
    let file_size_filter = |entry: &DirEntry| match &config.size_type {
        Some(size_type) => {
            let metadata = entry.metadata().unwrap();
            let size = metadata.size() / size_type.blksize
                + if metadata.size() % size_type.blksize != 0 {
                    1
                } else {
                    0
                };
            match size_type.cmp_flag {
                CmpFlag::Plus => size > size_type.size,
                CmpFlag::Minus => size < size_type.size,
                CmpFlag::None => size == size_type.size,
            }
        }
        None => true,
    };
    for path in config.paths {
        walk_dir(&path)
            .into_iter()
            .filter_map(|entry| match entry {
                Err(e) => {
                    eprintln!("{e}");
                    None
                }
                Ok(entry) => Some(entry),
            })
            .filter(name_filter)
            .filter(entry_type_filter)
            .filter(file_size_filter)
            .map(|entry| format!("{}", entry.path().display()))
            .for_each(|path| println!("{path}"));
    }
    Ok(())
}
