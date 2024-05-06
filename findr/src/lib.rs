use anyhow::Result;
use clap::{builder::PossibleValue, Parser, ValueEnum};
use regex::Regex;
use std::fmt::Debug;
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

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            EntryType::Dir => PossibleValue::new("d").into(),
            EntryType::File => PossibleValue::new("f").into(),
            EntryType::Link => PossibleValue::new("l").into(),
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
}

pub fn get_args() -> Result<Config> {
    let config = Config::try_parse()?;
    Ok(config)
}

pub fn run(config: Config) -> Result<()> {
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
    for path in config.paths {
        let mut walk_dir = WalkDir::new(path);
        if let Some(depth) = config.min_depth {
            walk_dir = walk_dir.min_depth(depth);
        }
        if let Some(depth) = config.max_depth {
            walk_dir = walk_dir.max_depth(depth);
        }
        walk_dir
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
            .map(|entry| format!("{}", entry.path().display()))
            .for_each(|path| println!("{path}"));
    }
    Ok(())
}
