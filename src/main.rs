use std::env;
use std::fs::{read_to_string, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use clap::{App, Arg};
use colored::Colorize;

mod toml_tokenizer;
use toml_tokenizer::{parse::Parse, TomlTokenizer};

//Takes a file path and reads its contents in as plain text
fn load_file_contents(path: &str) -> String {
    read_to_string(path).unwrap_or_else(|_| {
        let msg = format!("{} No file found at: {}", "ERROR:".red(), path);
        eprintln!("{}", msg);
        std::process::exit(1);
    })
}

fn load_toml_file(path: &PathBuf) -> Option<String> {
    //Check if a valid .toml filepath
    let path = path.to_str().unwrap_or_else(|| {
        let msg = format!("{} path could not be represented as str", "ERROR:".red());
        eprintln!("{}", msg);
        std::process::exit(1);
    });
    if !path.contains(".toml") {
        eprintln!(
            "{}",
            &format!("{} invalid path to .toml file: {}", "ERROR:".red(), path)
        );
        return None;
    }
    Some(load_file_contents(path))
}

// it would be nice to be able to check if the file had been saved recently
// or check if uncommited changes were present
fn write_file(mut path: PathBuf, tt: &TomlTokenizer) -> std::io::Result<()> {
    path.pop();
    path.push("test.toml");

    let mut fd = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;

    write!(fd, "{}", tt)
}

fn main() -> std::io::Result<()> {
    let included_headers: Vec<&str> = vec![
        "dependencies",
        "dev-dependencies",
        "build-dependencies",
        "workspace.members",
        "workspace.exclude",
    ];

    let matches = App::new("cargo-dep-sort")
        .author("Devin R <devin.ragotzy@gmail.com>")
        .about("Helps ensure Cargo.toml dependency list is sorted.")
        .arg(
            Arg::with_name("cwd")
                .value_name("CWD")
                .help("Sets cwd, must contain Cargo.toml")
                .index(1),
        )
        .arg(
            Arg::with_name("write")
                .short("w")
                .long("write")
                .help("rewrites Cargo.toml file so it is lexically sorted"),
        )
        .get_matches();

    let cwd = env::current_dir().unwrap_or_else(|e| {
        let msg = format!("{} No file found at: {}", "ERROR:".red(), e);
        eprintln!("{}", msg);
        std::process::exit(1);
    });
    // either default cwd or from user
    let mut path = matches
        .value_of("cwd")
        .map_or(cwd, |s| PathBuf::from(s.to_owned()));

    if path.extension().is_none() {
        path.push("Cargo.toml");
    }

    // TODO make write to file
    let write_flag = matches.is_present("write");

    let toml_raw = match load_toml_file(&path) {
        Some(t) => t,
        None => std::process::exit(1),
    };

    // parses/to_token the toml for sort checking
    let mut tt = TomlTokenizer::parse(&toml_raw).unwrap_or_else(|e| {
        let msg = format!("{} No file found at: {}", "ERROR:".red(), e);
        eprintln!("{}", msg);
        std::process::exit(1);
    });

    println!("{}", tt);

    //Check if appropriate tables in file are sorted
    for header in included_headers.iter() {
        tt.sort_items(header);
        tt.sort_nested(header);
    }

    println!("{}", tt);
    if write_flag {
        write_file(path, &tt).unwrap_or_else(|e| {
            let msg = format!("{} Failed to rewrite file: {}", "ERROR:".red(), e);
            eprintln!("{}", msg);
            std::process::exit(1);
        });
    }

    if !tt.was_sorted() {
        println!(
            "{} dependencies are sorted!",
            "Success".bold().bright_green()
        );
        std::process::exit(0);
    } else {
        eprintln!("{} dependencies are not sorted", "Failure".bold().red());
        std::process::exit(1);
    }
}
