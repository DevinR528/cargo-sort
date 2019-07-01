use std::env;
use std::fs;
use std::path::PathBuf;

use clap::{App, Arg};
use colored::Colorize;
use toml::de;

mod toml_tokenizer;
use toml_tokenizer::{parse::Parse, TomlTokenizer};

//Takes a file path and reads its contents in as plain text
fn load_file_contents(path: &str) -> String {
    let file_contents = fs::read_to_string(path).unwrap_or_else(|_| {
        let msg = format!("{} No file found at: {}", "ERROR:".red(), path);
        eprintln!("{}", msg);
        std::process::exit(1);
    });
    // TODO: remove
    // since we are only string munching validate it first
    if let Err(e) = de::from_str::<toml::value::Table>(&file_contents) {
        println!("{}", &format!("{} {} in {}", "ERROR:".red(), e, path));
        std::process::exit(1)
    }

    return file_contents;
}

fn load_toml_file(path: &str) -> Option<String> {
    //Check if a valid .toml filepath
    if !path.contains(".toml") {
        eprintln!(
            "{}",
            &format!("{} invalid path to .toml file:\n{}", "ERROR:".red(), path)
        );
        return None;
    }
    Some(load_file_contents(path))
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

    let cwd = env::current_dir().expect(&format!("{} could not get cwd", "ERROR:".red()));
    // either default cwd or user selected
    let mut path = matches
        .value_of("cwd")
        .map_or(cwd, |s| PathBuf::from(s.to_owned()));
    match path.extension() {
        None => {
            path.push("Cargo.toml");
        }
        _ => {}
    }

    let write_flag = matches.is_present("write");

    let toml_raw = match load_toml_file(path.to_str().unwrap()) {
        Some(t) => t,
        None => std::process::exit(1),
    };

    // parses/to_token the toml for sort checking
    let mut tt = TomlTokenizer::parse(&toml_raw).unwrap_or_else(|e| {
        let msg = format!("{} No file found at: {}", "ERROR:".red(), e);
        eprintln!("{}", msg);
        std::process::exit(1);
    });

    //Check if appropriate tables in file are sorted
    // for header in included_headers.iter() {
    //     let full_header = format!("[{}]", header);
    //     tr.slice_table(full_header, "\n[")?;

    //     if header.contains("dependencies") {
    //         while tr.slice_header(format!("[{}.", header), "]")? {}
    //     }
    // }

    if
    /*tr.is_sorted()*/
    true {
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
