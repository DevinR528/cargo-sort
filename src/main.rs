use std::env;
use std::fs::{read_to_string, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use clap::{App, Arg};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

mod toml_tokenizer;
use toml_tokenizer::{parse::Parse, TomlTokenizer};

const HEADERS: [&str; 5] = [
    "dependencies",
    "dev-dependencies",
    "build-dependencies",
    "workspace.members",
    "workspace.exclude",
];

fn write_err(msg: &str) -> std::io::Result<()> {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
    write!(stderr, "Failure: ")?;
    stderr.reset()?;
    writeln!(stderr, "{}", msg)
}

fn write_succ(msg: &str) -> std::io::Result<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
    write!(stdout, "Success: ")?;
    stdout.reset()?;
    writeln!(stdout, "{}", msg)
}

//Takes a file path and reads its contents in as plain text
fn load_file_contents(path: &str) -> String {
    read_to_string(path).unwrap_or_else(|_| {
        let msg = format!("No file found at: {}", path);
        write_err(&msg).unwrap();
        std::process::exit(1);
    })
}

fn load_toml_file(path: &PathBuf) -> String {
    //Check if a valid .toml filepath
    let path = path.to_str().unwrap_or_else(|| {
        write_err("path could not be represented as str").unwrap();
        std::process::exit(1)
    });
    if !path.contains(".toml") {
        let msg = format!("invalid path to .toml file: {}", path);
        write_err(&msg).unwrap();
        std::process::exit(1)
    }
    load_file_contents(path)
}

// TODO:
// it would be nice to be able to check if the file had been saved recently
// or check if uncommited changes were present
fn write_file(path: &PathBuf, tt: &TomlTokenizer) -> std::io::Result<()> {
    let mut fd = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;

    write!(fd, "{}", tt)
}

fn check_toml(path: &str, matches: &clap::ArgMatches) -> bool {
    let mut path = PathBuf::from(path);
    if path.extension().is_none() {
        path.push("Cargo.toml");
    }

    let toml_raw = load_toml_file(&path);

    // parses/to_tokens the toml file for sort checking
    let mut tt = TomlTokenizer::parse(&toml_raw).unwrap_or_else(|e| {
        let msg = format!("TOML parse error: {}", e);
        write_err(&msg).unwrap();
        std::process::exit(1);
    });

    // check if appropriate tables in file are sorted
    for header in HEADERS.iter() {
        tt.sort_items(header);
        tt.sort_nested(header);
    }

    if matches.is_present("CRLF") {
        tt.set_eol("\r\n");
    }

    if matches.is_present("print") {
        print!("{}", tt);
        if !matches.is_present("write") {
            return true;
        }
    }

    if matches.is_present("write") {
        write_file(&path, &tt).unwrap_or_else(|e| {
            let msg = format!("failed to rewrite file: {:?}", e);
            write_err(&msg).unwrap();
        });
        let msg = format!("dependencies are sorted for {:?}", path);
        write_succ(&msg).unwrap();
        return true;
    }

    if !tt.was_sorted() {
        let msg = format!("dependencies are sorted for {:?}", path);
        write_succ(&msg).unwrap();
        true
    } else {
        let msg = format!("dependencies are not sorted for {:?}", path);
        write_err(&msg).unwrap();
        false
    }
}

fn main() -> std::io::Result<()> {
    let matches = App::new("Cargo Sort Check")
        .author("Devin R <devin.ragotzy@gmail.com>")
        .about("Ensure Cargo.toml dependency tables are sorted.")
        .usage("cargo-sort-ck [FLAGS] [CWD]")
        .arg(
            Arg::with_name("cwd")
                .value_name("CWD")
                .multiple(true)
                .help("Sets cwd, must contain Cargo.toml"),
        )
        .arg(
            Arg::with_name("write")
                .short("w")
                .long("write")
                .help("rewrites Cargo.toml file so it is lexically sorted"),
        )
        .arg(
            Arg::with_name("print")
                .short("p")
                .long("print")
                .help("prints Cargo.toml, lexically sorted, to the screen"),
        )
        .arg(
            Arg::with_name("CRLF")
                .long("crlf")
                .help("output uses windows style line endings"),
        )
        .get_matches();

    let cwd = env::current_dir().unwrap_or_else(|e| {
        let msg = format!("No file found at: {}", e);
        write_err(&msg).unwrap();
        std::process::exit(1);
    });
    // either default cwd or from user
    let path = matches.values_of("cwd").map_or(cwd, |s| {
        let dirs: Vec<&str> = s.collect();
        if dirs.len() == 1 {
            PathBuf::from(dirs[0])
        } else {
            let mut flag = true;
            dirs.iter()
                .map(|path| check_toml(path, &matches))
                .for_each(|sorted| {
                    if !sorted {
                        flag = false;
                    }
                });
            if flag {
                std::process::exit(0)
            } else {
                std::process::exit(1)
            }
        }
    });

    if check_toml(path.to_str().unwrap(), &matches) {
        std::process::exit(0)
    } else {
        std::process::exit(1)
    }
}
