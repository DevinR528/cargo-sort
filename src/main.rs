use std::{
    borrow::Cow,
    env,
    fs::{read_to_string, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use clap::{crate_name, crate_version, App, Arg};
use fmt::Config;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use toml_edit::{Document, Item};

mod fmt;
mod sort;
mod toml_edit;

#[rustfmt::skip]
const EXTRA_HELP: &str =
"NOTE: formatting is applied after the check for sorting so
      sorted but unformatted toml will not cause a failure";

type IoResult<T> = Result<T, Box<dyn std::error::Error>>;

fn write_err(msg: &str) -> IoResult<()> {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
    write!(stderr, "Failure: ")?;
    stderr.reset()?;
    writeln!(stderr, "{}", msg).map_err(Into::into)
}

fn write_succ(msg: &str) -> IoResult<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
    write!(stdout, "Success: ")?;
    stdout.reset()?;
    writeln!(stdout, "{}", msg).map_err(Into::into)
}

// TODO:
// it would be nice to be able to check if the file had been saved recently
// or check if uncommitted changes were present
fn write_file<P: AsRef<Path>>(path: P, toml: &str) -> IoResult<()> {
    let mut fd = OpenOptions::new().write(true).create(true).truncate(true).open(path)?;
    write!(fd, "{}", toml).map_err(Into::into)
}

fn check_toml(
    path: &str,
    matches: &clap::ArgMatches<'_>,
    config: &Config,
) -> IoResult<bool> {
    let mut path = PathBuf::from(path);
    if path.extension().is_none() {
        path.push("Cargo.toml");
    }

    let toml_raw = read_to_string(&path)
        .map_err(|_| format!("No file found at: {}", path.display()))?;

    let mut sorted =
        sort::sort_toml(&toml_raw, sort::MATCHER, matches.is_present("grouped"));
    let mut sorted_str = sorted.to_string_in_original_order();
    let is_sorted = toml_raw == sorted_str;

    // if no-format is not found apply formatting
    if !matches.is_present("no-format") {
        fmt::fmt_toml(&mut sorted, config);
        sorted_str = sorted.to_string_in_original_order();
    }

    if config.crlf && !sorted_str.contains("\r\n") {
        sorted_str = sorted_str.replace("\n", "\r\n")
    }

    if matches.is_present("print") {
        print!("{}", sorted_str);
        return Ok(true);
    }

    if matches.is_present("check") {
        if is_sorted {
            write_succ(&format!("dependencies are sorted for {:?}", path))?;
        } else {
            write_err(&format!("dependencies are not sorted for {:?}", path))?;
        }
        return Ok(is_sorted);
    }

    write_file(&path, &sorted_str)?;
    write_succ(&format!("dependencies are now sorted for {:?}", path))?;

    Ok(true)
}

fn _main() -> IoResult<()> {
    let matches =
        App::new(crate_name!())
            .author("Devin R <devin.ragotzy@gmail.com>")
            .version(crate_version!())
            .about("Ensure Cargo.toml dependency tables are sorted.")
            .arg(
                Arg::with_name("cwd")
                    .value_name("CWD")
                    .multiple(true)
                    .help("sets cwd, must contain a Cargo.toml file"),
            )
            .arg(Arg::with_name("check").short("c").long("check").help(
                "non-zero exit if Cargo.toml is unsorted, overrides default behavior",
            ))
            .arg(
                Arg::with_name("print")
                    .short("p")
                    .long("print")
                    // No printing if we are running a --check
                    .conflicts_with("check")
                    .help("prints Cargo.toml, lexically sorted, to stdout"),
            )
            .arg(
                Arg::with_name("no-format")
                    .short("n")
                    .long("no-format")
                    // Force this arg to be present if --check is
                    .default_value_if("check", None, "")
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
            .after_help(EXTRA_HELP)
            .get_matches();

    let cwd =
        env::current_dir().map_err(|e| format!("no current directory found: {}", e))?;
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

        let raw_toml = read_to_string(&path)
            .map_err(|_| format!("no file found at: {}", path.display()))?;
        let toml = raw_toml.parse::<Document>()?;
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
                    for entry in glob::glob(&format!("{}/{}", dir, member))
                        .unwrap_or_else(|e| {
                            write_err(&format!("Glob failed: {}", e)).unwrap();
                            std::process::exit(1);
                        })
                    {
                        let path = entry?;
                        filtered_matches.push(Cow::Owned(path.display().to_string()));
                    }
                } else {
                    filtered_matches.push(Cow::Owned(format!("{}/{}", dir, member)));
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
        if !(sorted?) {
            flag = false;
        }
    }

    if flag { std::process::exit(0) } else { std::process::exit(1) }
}

fn main() {
    _main().unwrap_or_else(|e| {
        write_err(&e.to_string()).unwrap();
        std::process::exit(1);
    })
}

// #[test]
// fn fuzzy_fail() {
//     for file in std::fs::read_dir("out/default/crashes").unwrap() {
//         let path = file.unwrap().path();
//         println!("{}", path.display());
//         let s = read_to_string(&path).unwrap().replace("\r", "");
//         let mut toml = sort::sort_toml(&s, sort::MATCHER, false);
//         fmt::fmt_toml(&mut toml, &fmt::Config::default());
//         print!("{}", s);
//         s.parse::<Document>().unwrap();
//     }
// }
