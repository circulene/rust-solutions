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

fn display_entry(path: &Path, depth: u32, is_last: bool) -> Result<()> {
    if depth > 1 {
        print!("│   ");
        for _ in 1..depth - 1 {
            print!("    ");
        }
    }
    let mut entry_name = path.file_name().unwrap().to_string_lossy();
    if path.is_symlink() {
        entry_name
            .to_mut()
            .push_str(format!(" -> {}", path.read_link()?.display()).as_str());
    }
    if !is_last {
        println!("├── {}", entry_name);
    } else {
        println!("└── {}", entry_name);
    }
    Ok(())
}

fn walk_dir(root: &Path, depth: u32) -> Result<EntryCounter> {
    let mut entries = root
        .read_dir()?
        .filter_map(|res| res.ok())
        .map(|e| e.path())
        .collect::<Vec<_>>();
    entries.sort();
    let mut counter = EntryCounter::new();

    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == entries.len() - 1;
        display_entry(entry.as_path(), depth + 1, is_last)?;
        counter.inc(entry.as_path());
        if entry.is_dir() {
            let sub_counter = walk_dir(entry.as_path(), depth + 1)?;
            counter.sum(&sub_counter);
        }
    }

    Ok(counter)
}

fn main() {
    let config = Config::parse();

    println!("{}", &config.path);
    let root = Path::new(&config.path);
    match walk_dir(root, 0) {
        Err(err) => eprintln!("{err}"),
        Ok(mut counter) => {
            counter.inc(root);
            println!("\n{} directories, {} files", counter.dir, counter.file);
        }
    }
}
