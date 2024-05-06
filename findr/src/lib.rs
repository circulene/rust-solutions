use anyhow::Result;
use clap::{builder::PossibleValue, Parser, ValueEnum};
use regex::Regex;
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
}

pub fn get_args() -> Result<Config> {
    let config = Config::try_parse()?;
    Ok(config)
}

fn is_name_matched(names: &[Regex], entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    if !names.is_empty() {
        names.iter().any(|x| x.is_match(&name))
    } else {
        true
    }
}

fn is_entry_type_matched(entry_types: &[EntryType], entry: &DirEntry) -> bool {
    let file_type = entry.file_type();
    if !entry_types.is_empty() {
        entry_types.iter().any(|x| match x {
            EntryType::Dir => file_type.is_dir(),
            EntryType::File => file_type.is_file(),
            EntryType::Link => file_type.is_symlink(),
        })
    } else {
        true
    }
}

pub fn run(config: Config) -> Result<()> {
    for path in config.paths {
        for entry in WalkDir::new(path) {
            match entry {
                Err(e) => eprintln!("{e}"),
                Ok(entry) => {
                    if !is_name_matched(&config.names, &entry) {
                        continue;
                    }
                    if !is_entry_type_matched(&config.entry_types, &entry) {
                        continue;
                    }
                    println!("{}", entry.path().display());
                }
            }
        }
    }
    Ok(())
}
