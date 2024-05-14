use anyhow::Result;
use clap::Parser;
use std::path::Path;

#[derive(Parser)]
struct Config {
    #[arg(value_name = "PATH", default_value = ".")]
    path: String,
}

struct EntryCounter {
    dir: u32,
    file: u32,
}

impl EntryCounter {
    fn new() -> EntryCounter {
        EntryCounter { dir: 0, file: 0 }
    }

    fn inc(&mut self, path: &Path) {
        if path.is_dir() {
            self.dir += 1;
        } else {
            self.file += 1;
        }
    }

    fn sum(&mut self, counter: &EntryCounter) {
        self.dir += counter.dir;
        self.file += counter.file;
    }
}

fn display_entry(path: &Path, prefix: &str, is_last: bool) -> Result<()> {
    let mut entry_name = path.file_name().unwrap().to_string_lossy();
    if path.is_symlink() {
        entry_name
            .to_mut()
            .push_str(format!(" -> {}", path.read_link()?.display()).as_str());
    }
    if !is_last {
        println!("{}├── {}", prefix, entry_name);
    } else {
        println!("{}└── {}", prefix, entry_name);
    }
    Ok(())
}

fn walk_dir(root: &Path, prefix: &str) -> Result<EntryCounter> {
    let mut entries = root
        .read_dir()?
        .filter_map(|res| res.ok())
        .map(|e| e.path())
        .collect::<Vec<_>>();
    entries.sort();
    let mut counter = EntryCounter::new();

    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == entries.len() - 1;
        display_entry(entry.as_path(), prefix, is_last)?;
        counter.inc(entry.as_path());
        if entry.is_dir() {
            let mut new_prefix = prefix.to_string();
            new_prefix.push_str(if is_last { "    " } else { "│   " });
            let sub_counter = walk_dir(entry.as_path(), new_prefix.as_str())?;
            counter.sum(&sub_counter);
        }
    }

    Ok(counter)
}

fn main() {
    let config = Config::parse();

    println!("{}", &config.path);
    let root = Path::new(&config.path);
    match walk_dir(root, "") {
        Err(err) => eprintln!("{err}"),
        Ok(mut counter) => {
            counter.inc(root);
            println!("\n{} directories, {} files", counter.dir, counter.file);
        }
    }
}
