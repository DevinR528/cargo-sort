use std::fs;
use std::fs::{ File, OpenOptions };
use std::env;
use std::path::{ PathBuf, };


use clap::{App, Arg};
use colored::{ Colorize };
use regex::Regex;

mod utils;
mod reader;
use reader::TomlReader;

//Checks if filepath points to a .toml file
fn is_toml_filepath(filepath: &str) -> bool {
    let toml_filepath_re = Regex::new(r"^.*\.toml$").unwrap();
    return toml_filepath_re.is_match(filepath);
}

//Takes a file path and reads its contents in as plain text
fn load_file_contents(filepath: &str) -> String {
    let file_contents =
        fs::read_to_string(filepath)
        .expect(&format!("{} Something went wrong reading the file", "ERROR:".red()));
    return file_contents;
}

fn load_toml_file(toml_filepath: &str) -> Option<String> {
    //Check if a valid .toml filepath
    if !is_toml_filepath(toml_filepath) {
        eprintln!("{}", &format!("{} detected invalid path to .toml file:\n{}",
            "ERROR:".red(),
            toml_filepath
        ));
        return None
    }
    //Fetch toml data
    Some(load_file_contents(toml_filepath))
}

/// Returns the string if it needed sorting else None
/// sorts including version
fn check_table_sorted(toml_table: &toml::value::Table) -> bool {
    let dep_table: Vec<&str> = toml_table.iter()
        .map(|(k, _v)| k)
        .filter(|k| k != &"")
        .map(AsRef::as_ref)
        .collect();
    
    let mut sorted_table = dep_table.clone();
    sorted_table.sort_unstable();

    dep_table == sorted_table
}

//TODO: implement unit/integration tests for all major functions
//TODO: write functions to write a properly sorted Cargo.toml file to disk

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
        .author("Jordan Poles <jpdev.noreply@gmail.com>")
        .about("Helps ensure sorting of Cargo.toml file dependency list")
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

    println!("{:#?}", path);

    let mut toml_raw = match load_toml_file(path.to_str().unwrap()) {
        Some(t) => t,
        None => std::process::exit(1),
    };
    
    let mut tr = TomlReader::new(&mut toml_raw);
    //Check if appropriate tables in file are sorted
    for header in included_headers.iter() {
        let full_header = format!("[{}]", header);
        tr.slice_table(&full_header, "\n[")?;

        if header.contains("dependencies") {
            while tr.slice_header(&format!("{}.", header), "]")? {}
        }
    }

    if tr.is_sorted() {
        println!("{} dependencies are sorted!", "Success".bold().bright_green());
        std::process::exit(0);
    } else {
        println!("{} dependencies are not sorted", "Failure".bold().red());
        std::process::exit(1);
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_toml_filepath() {
        assert!(is_toml_filepath("/cargo.toml"));
        assert!(!is_toml_filepath("cargo.tomls"));
    }
}
