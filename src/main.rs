use std::{
    borrow::Cow,
    env,
    fmt::Display,
    fs::{read_to_string, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use clap::{
    crate_authors, crate_name, crate_version, parser::ValueSource, Arg, ArgAction,
    ArgMatches, Command,
};
use fmt::Config;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use toml_edit::{Document, Item};

mod fmt;
mod sort;

const EXTRA_HELP: &str = "\
    NOTE: formatting is applied after the check for sorting so \
          sorted but unformatted toml will not cause a failure";

type IoResult<T> = Result<T, Box<dyn std::error::Error>>;

fn flag_set(flag: &str, matches: &ArgMatches) -> bool {
    matches!(
        matches.value_source(flag),
        Some(ValueSource::CommandLine | ValueSource::EnvVariable)
    )
}

fn write_red<S: Display>(highlight: &str, msg: S) -> IoResult<()> {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
    write!(stderr, "{}", highlight)?;
    stderr.reset()?;
    writeln!(stderr, "{}", msg).map_err(Into::into)
}

fn write_green<S: Display>(highlight: &str, msg: S) -> IoResult<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
    write!(stdout, "{}", highlight)?;
    stdout.reset()?;
    writeln!(stdout, "{}", msg).map_err(Into::into)
}

fn write_file<P: AsRef<Path>>(path: P, toml: &str) -> IoResult<()> {
    let mut fd = OpenOptions::new().write(true).create(true).truncate(true).open(path)?;
    write!(fd, "{}", toml).map_err(Into::into)
}

fn check_toml(path: &str, matches: &ArgMatches, config: &Config) -> IoResult<bool> {
    let mut path = PathBuf::from(path);
    if path.extension().is_none() {
        path.push("Cargo.toml");
    }

    let krate = path.components().nth_back(1).ok_or("No crate folder found")?.as_os_str();

    write_green("Checking ", format!("{}...", krate.to_string_lossy()))?;

    let toml_raw = read_to_string(&path)
        .map_err(|_| format!("No file found at: {}", path.display()))?;

    let mut sorted = sort::sort_toml(
        &toml_raw,
        sort::MATCHER,
        flag_set("grouped", matches),
        &config.table_order,
    );
    let mut sorted_str = sorted.to_string();
    let is_sorted = toml_raw == sorted_str;

    let is_formatted =
        // if no-format is not found apply formatting
        if !flag_set("no-format", matches) || flag_set("check-format", matches) {
            let original = sorted_str.clone();
            fmt::fmt_toml(&mut sorted, config);
            sorted_str = sorted.to_string();
            original == sorted_str
        } else {
            true
        };

    if config.crlf && !sorted_str.contains("\r\n") {
        sorted_str = sorted_str.replace('\n', "\r\n")
    }

    if flag_set("print", matches) {
        print!("{}", sorted_str);
        return Ok(true);
    }

    if flag_set("check", matches) {
        if !is_sorted {
            write_red(
                "error: ",
                format!("Dependencies for {} are not sorted", krate.to_string_lossy()),
            )?;
        }

        if !is_formatted {
            write_red(
                "error: ",
                format!("Cargo.toml for {} is not formatted", krate.to_string_lossy()),
            )?;
        }

        return Ok(is_sorted && is_formatted);
    }

    write_file(&path, &sorted_str)?;
    write_green(
        "Finished: ",
        format!("Cargo.toml for {:?} has been rewritten", krate.to_string_lossy()),
    )?;

    Ok(true)
}

fn _main() -> IoResult<()> {
    let matches =
        Command::new(crate_name!())
            .author(crate_authors!())
            .version(crate_version!())
            .about("Ensure Cargo.toml dependency tables are sorted.")
            .arg(
                Arg::new("cwd")
                    .value_name("CWD")
                    .action(ArgAction::Append)
                    .help("sets cwd, must contain a Cargo.toml file"),
            )
            .arg(Arg::new("check").short('c').long("check")
            .action(ArgAction::SetTrue)
            .help(
                "Returns non-zero exit code if Cargo.toml is unsorted, overrides default behavior",
            ))
            .arg(
                Arg::new("print")
                    .short('p')
                    .long("print")
                    .action(ArgAction::SetTrue)
                    // No printing if we are running a --check
                    .conflicts_with("check")
                    .help("Prints Cargo.toml, lexically sorted, to stdout"),
            )
            .arg(
                Arg::new("no-format")
                    .short('n')
                    .long("no-format")
                    .action(ArgAction::SetTrue)
                    .help("Skips formatting after sorting"),
            )
            .arg(
                Arg::new("check-format")
                    .requires("check")
                    .long("check-format")
                    .action(ArgAction::SetTrue)
                    .help("Also returns non-zero exit code if formatting changes"),
            )
            .arg(
                Arg::new("workspace")
                    .short('w')
                    .long("workspace")
                    .action(ArgAction::SetTrue)
                    .help("Checks every crate in a workspace"),
            )
            .arg(
                Arg::new("grouped")
                    .short('g')
                    .long("grouped")
                    .action(ArgAction::SetTrue)
                    .help("Keep blank lines when sorting groups of key value pairs"),
            )
            .arg(
                Arg::new("order")
                    .short('o')
                    .long("order")
                    .action(ArgAction::Append)
                    .value_delimiter(',')
                    .help("List the order tables should be written out (--order package,dependencies,features)"),
            )
            .after_help(EXTRA_HELP)
            .get_matches();

    let cwd =
        env::current_dir().map_err(|e| format!("no current directory found: {}", e))?;
    let dir = cwd.to_string_lossy();

    // remove "sort" when invoked `cargo sort` sort is the first arg
    // https://github.com/rust-lang/cargo/issues/7653
    let (is_posible_workspace, mut filtered_matches) =
        matches.get_many::<String>("cwd").map_or((true, vec![dir.clone()]), |s| {
            let args = s.filter(|it| *it != "sort").map(Into::into).collect::<Vec<_>>();
            if args.is_empty() { (true, vec![dir]) } else { (args.len() == 1, args) }
        });

    if flag_set("workspace", &matches) && is_posible_workspace {
        let dir = filtered_matches[0].to_string();
        let mut path = PathBuf::from(&dir);
        if path.extension().is_none() {
            path.push("Cargo.toml");
        }

        let raw_toml = read_to_string(&path)
            .map_err(|_| format!("no file found at: {}", path.display()))?;

        let toml = raw_toml.parse::<Document>()?;
        let workspace = toml.get("workspace");
        if let Some(Item::Table(ws)) = workspace {
            // The workspace excludes, used to filter members by
            let excludes: Vec<&str> =
                ws.get("exclude").map_or_else(Vec::new, array_string_members);
            for member in ws.get("members").map_or_else(Vec::new, array_string_members) {
                // TODO: a better test wether to glob?
                if member.contains('*') || member.contains('?') {
                    'globs: for entry in glob::glob(&format!("{}/{}", dir, member))
                        .unwrap_or_else(|e| {
                            write_red("error: ", format!("Glob failed: {}", e)).unwrap();
                            std::process::exit(1);
                        })
                    {
                        let path = entry?;

                        // The `check_toml` function expects only folders that it appends
                        // `Cargo.toml` onto
                        if path.is_file() {
                            continue;
                        }

                        // Since the glob function gives us actual paths we need to only
                        // check if the relevant parts match so we can't just do
                        // `excludes.contains(..)`
                        let path_str = path.to_string_lossy();
                        for excl in &excludes {
                            if path_str.ends_with(excl) {
                                continue 'globs;
                            }
                        }

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
    let mut config = read_to_string(&cwd)
        .or_else(|_err| {
            cwd.pop();
            cwd.push(".tomlfmt.toml");
            read_to_string(&cwd)
        })
        .unwrap_or_default()
        .parse::<Config>()?;

    if let Some(ordering) =
        matches.get_many::<String>("order").map(|v| v.collect::<Vec<_>>())
    {
        config.table_order = ordering.into_iter().map(|s| s.to_string()).collect();
    }

    let mut flag = true;
    for sorted in filtered_matches.iter().map(|path| check_toml(path, &matches, &config))
    {
        if !(sorted?) {
            flag = false;
        }
    }

    if flag { std::process::exit(0) } else { std::process::exit(1) }
}

fn array_string_members(value: &toml_edit::Item) -> Vec<&str> {
    value.as_array().into_iter().flatten().filter_map(|s| s.as_str()).collect()
}

fn main() {
    _main().unwrap_or_else(|e| {
        write_red("error: ", e).unwrap();
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
