use std::env;
use std::fs::{read_to_string, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use clap::{App, Arg};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use toml_parse::{parse_it, sort_toml_items, Matcher, Formatter, SyntaxNodeExtTrait};

const HEADERS: [&str; 3] = [
    "[dependencies]",
    "[dev-dependencies]",
    "[build-dependencies]",
];

const HEADER_SEG: [&str; 3] = [
    "dependencies.",
    "dev-dependencies.",
    "build-dependencies.",
];

const MATCHER: Matcher<'_> = Matcher {
    heading: &HEADERS,
    segmented: &HEADER_SEG,
    heading_key: &[("[workspace]", "members"), ("[workspace]", "exclude")],
};

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
fn write_file(path: &PathBuf, toml: &str) -> std::io::Result<()> {
    let mut fd = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;

    write!(fd, "{}", toml)
}

fn check_toml(path: &str, matches: &clap::ArgMatches) -> bool {
    let mut path = PathBuf::from(path);
    if path.extension().is_none() {
        path.push("Cargo.toml");
    }

    let toml_raw = load_toml_file(&path);

    // parses the toml file for sort checking
    let tt = parse_it(&toml_raw).unwrap_or_else(|e| {
        let msg = format!("toml parse error: {}", e);
        write_err(&msg).unwrap();
        std::process::exit(1);
    }).syntax();

    // check if appropriate tables in file are sorted
    let sorted = sort_toml_items(&tt, &MATCHER);
    let was_sorted = !sorted.deep_eq(&tt);

    let fmted = Formatter::new(&sorted).format().to_string();

    if matches.is_present("print") {
        print!("{}", fmted);
        if !matches.is_present("write") {
            return true;
        }
    }

    if matches.is_present("write") {
        write_file(&path, &fmted).unwrap_or_else(|e| {
            let msg = format!("failed to rewrite file: {:?}", e);
            write_err(&msg).unwrap();
        });
        let msg = format!("dependencies are now sorted for {:?}", path);
        write_succ(&msg).unwrap();
        return true;
    }

    if was_sorted {
        let msg = format!("dependencies are not sorted for {:?}", path);
        write_err(&msg).unwrap();
        false
    } else {
        let msg = format!("dependencies are sorted for {:?}", path);
        write_succ(&msg).unwrap();
        true
    }
}

fn main() {
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
                .help("output uses windows style line endings (\\r\\n)"),
        )
        .get_matches();

    let cwd = env::current_dir()
        .unwrap_or_else(|e| {
            let msg = format!("no current directory found: {}", e);
            write_err(&msg).unwrap();
            std::process::exit(1);
        });
    let dir = cwd.to_str()
        .unwrap_or_else(|| {
            let msg = format!("could not represent path as string");
            write_err(&msg).unwrap();
            std::process::exit(1);
        });

    // remove "sort-ck" when invoked `cargo sort-ck` sort-ck is the first arg
    // https://github.com/rust-lang/cargo/issues/7653
    let filtered_matches = matches.values_of("cwd").map_or(vec![dir], |s| {
        let args = s.filter(|it| *it != "sort-ck").collect::<Vec<&str>>();
        if args.is_empty() {
            vec![dir]
        } else {
            args
        }
    });

    let mut flag = true;
    filtered_matches.iter()
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
