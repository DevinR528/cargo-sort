use std::{
    borrow::Cow,
    env,
    fs::{read_to_string, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use clap::{App, Arg};
use fmt::Config;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use toml_edit::{Document, Item};

mod fmt;
mod sort;
mod toml_edit;

const VERSION: &str = include_str!("../Cargo.toml");

/// Each `Matcher` field when matched to a heading or key token
/// will be matched with `.contains()`.
pub struct Matcher<'a> {
    /// Toml headings with braces `[heading]`.
    pub heading: &'a [&'a str],
    /// Toml heading with braces `[heading]` and the key
    /// of the array to sort.
    pub heading_key: &'a [(&'a str, &'a str)],
}

const HEADERS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

const MATCHER: Matcher<'_> = Matcher {
    heading: &HEADERS,
    heading_key: &[("workspace", "members"), ("workspace", "exclude")],
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

// TODO:
// it would be nice to be able to check if the file had been saved recently
// or check if uncommitted changes were present
fn write_file<P: AsRef<Path>>(path: P, toml: &str) -> std::io::Result<()> {
    let mut fd = OpenOptions::new().write(true).create(true).truncate(true).open(path)?;
    write!(fd, "{}", toml)
}

fn check_toml(path: &str, matches: &clap::ArgMatches<'_>, config: &Config) -> bool {
    let mut path = PathBuf::from(path);
    if path.extension().is_none() {
        path.push("Cargo.toml");
    }

    let toml_raw = read_to_string(&path).unwrap_or_else(|_| {
        write_err(&format!("No file found at: {}", path.display())).unwrap();
        std::process::exit(1);
    });

    let mut sorted = sort::sort_toml(&toml_raw, MATCHER, matches.is_present("grouped"));
    let mut sorted_str = sorted.to_string_in_original_order();
    let is_sorted = toml_raw == sorted_str;

    if matches.is_present("no-format") {
    } else {
        fmt::fmt_toml(&mut sorted, config);
        sorted_str = sorted.to_string_in_original_order();
    }

    if matches.is_present("print") {
        if config.crlf {
            sorted_str = sorted_str.replace("\n", "\r\n")
        }
        print!("{}", sorted_str);
        if !matches.is_present("check") {
            return true;
        }
    }

    if matches.is_present("check") {
        if is_sorted {
            write_succ(&format!("dependencies are sorted for {:?}", path)).unwrap();
        } else {
            write_err(&format!("dependencies are not sorted for {:?}", path)).unwrap();
        }
        return is_sorted;
    }

    if config.crlf {
        sorted_str = sorted_str.replace("\n", "\r\n")
    }
    write_file(&path, &sorted_str).unwrap_or_else(|e| {
        write_err(&format!("failed to rewrite file: {:?}", e)).unwrap();
    });
    write_succ(&format!("dependencies are now sorted for {:?}", path)).unwrap();

    true
}

fn main() {
    let matches = App::new("cargo sort")
        .author("Devin R <devin.ragotzy@gmail.com>")
        .version(&VERSION[41..46])
        .about("Ensure Cargo.toml dependency tables are sorted.")
        .usage("cargo-sort [FLAGS] [CWD]")
        .arg(
            Arg::with_name("cwd")
                .value_name("CWD")
                .multiple(true)
                .help("sets cwd, must contain a Cargo.toml file"),
        )
        .arg(
            Arg::with_name("check")
                .short("c")
                .long("check")
                .overrides_with_all(&["print", "format", "grouped"])
                .help("exit with non-zero if Cargo.toml is unsorted, overrides printing flags"),
        )
        .arg(
            Arg::with_name("print")
                .short("p")
                .long("print")
                .help("prints Cargo.toml, lexically sorted, to the screen"),
        )
        .arg(
            Arg::with_name("format")
                .short("f")
                .long("format")
                .help("formats the given Cargo.toml according to tomlfmt.toml"),
        )
        .arg(
            Arg::with_name("workspace")
                .short("w")
                .long("workspace")
                .help("checks every crate in a workspace"),
        )
        .arg(
            Arg::with_name("grouped")
                .short("g")
                .long("grouped")
                .help("when sorting groups of key value pairs blank lines are kept"),
        )
        .get_matches();

    let cwd = env::current_dir().unwrap_or_else(|e| {
        write_err(&format!("no current directory found: {}", e)).unwrap();
        std::process::exit(1);
    });
    let dir = cwd.to_string_lossy();

    // remove "sort" when invoked `cargo sort` sort is the first arg
    // https://github.com/rust-lang/cargo/issues/7653
    let (is_posible_workspace, mut filtered_matches) =
        matches.values_of("cwd").map_or((true, vec![dir.clone()]), |s| {
            let args = s.filter(|it| *it != "sort").map(Into::into).collect::<Vec<_>>();
            if args.is_empty() { (true, vec![dir]) } else { (args.len() == 1, args) }
        });

    if matches.is_present("workspace") && is_posible_workspace {
        let dir = filtered_matches[0].clone();
        let mut path = PathBuf::from(dir.as_ref());
        if path.extension().is_none() {
            path.push("Cargo.toml");
        }

        let raw_toml = read_to_string(&path).unwrap_or_else(|_| {
            write_err(&format!("No file found at: {}", path.display())).unwrap();
            std::process::exit(1);
        });
        let toml = raw_toml.parse::<Document>().unwrap_or_else(|e| {
            write_err(&e.to_string()).unwrap();
            std::process::exit(1);
        });
        let workspace = &toml["workspace"];
        if let Item::Table(ws) = workspace {
            for member in ws["members"]
                .as_array()
                .into_iter()
                .flat_map(|arr| arr.iter())
                .flat_map(|s| s.as_str())
            {
                // TODO: a better test wether to glob?
                if member.contains('*') || member.contains('?') {
                    for entry in
                        glob::glob(&format!("{}{}", dir, member)).unwrap_or_else(|e| {
                            write_err(&format!("Glob failed: {}", e)).unwrap();
                            std::process::exit(1);
                        })
                    {
                        match entry {
                            Ok(path) => filtered_matches
                                .push(Cow::Owned(path.display().to_string())),
                            Err(e) => {
                                write_err(&format!("Glob failed: {}", e)).unwrap();
                                std::process::exit(1);
                            }
                        }
                    }
                } else {
                    filtered_matches.push(Cow::Owned(format!("{}{}", dir, member)));
                }
            }
        }
    }

    let mut cwd = cwd.clone();
    cwd.push("tomlfmt.toml");
    let config = read_to_string(&cwd)
        .or_else(|_err| {
            cwd.pop();
            cwd.push(".tomlfmt.toml");
            read_to_string(&cwd)
        })
        .unwrap_or_default()
        .parse::<Config>()
        .unwrap_or_else(|e| {
            write_err(&e.to_string()).unwrap();
            std::process::exit(1);
        });

    let mut flag = true;
    for sorted in filtered_matches.iter().map(|path| check_toml(path, &matches, &config))
    {
        if !sorted {
            flag = false;
        }
    }

    if flag { std::process::exit(0) } else { std::process::exit(1) }
}
