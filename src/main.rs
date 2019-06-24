use std::fs;
use std::env;
use std::path::{ PathBuf, };


use clap::{App, Arg};
use colored::{ Colorize };

mod reader;
use reader::TomlReader;

//Takes a file path and reads its contents in as plain text
fn load_file_contents(filepath: &str) -> String {
    let file_contents =
        fs::read_to_string(filepath)
        .expect(&format!("{} Something went wrong reading the file", "ERROR:".red()));
    return file_contents;
}

fn load_toml_file(path: &str) -> Option<String> {
    //Check if a valid .toml filepath
    if !path.contains(".toml") {
        eprintln!("{}", &format!("{} detected invalid path to .toml file:\n{}",
            "ERROR:".red(),
            path
        ));
        return None
    }
    //Fetch toml data
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
    //Instantiate command line args through clap
    let matches = App::new("cargo-dep-sort")
        .author("Devin R <devin.ragotzy@gmail.com>")
        .about("Helps ensure Cargo.toml dependency list is sorted.")
        .arg(Arg::with_name("cwd")
                .value_name("CWD")
                .help("Sets cwd, must contain Cargo.toml")
                .index(1))
        .get_matches();

    
    let cwd = env::current_dir()
        .expect(&format!("{} could not get cwd", "ERROR:".red()));

    // either default cwd or user selected
    let mut path = matches.value_of("cwd")
        .map_or(cwd, |s| PathBuf::from(s.to_owned()));
    match path.extension() {
        None => {
            path.push("Cargo.toml");
        },
        _ => {},
    }

    let mut toml_raw = match load_toml_file(path.to_str().unwrap()) {
        Some(t) => t,
        None => std::process::exit(1),
    };
    
    let mut tr = TomlReader::new(&mut toml_raw);
    //Check if appropriate tables in file are sorted
    for header in included_headers.iter() {
        let full_header = format!("[{}]", header);
        tr.slice_table(full_header, "\n[")?;

        if header.contains("dependencies") {
            while tr.slice_header(format!("[{}.", header), "]")? {}
        }
    }

    if tr.is_sorted() {
        println!("{} dependencies are sorted!", "Success".bold().bright_green());
        std::process::exit(0);
    } else {
        eprintln!("{} dependencies are not sorted", "Failure".bold().red());
        std::process::exit(1);
    }
}
