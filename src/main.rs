use std::{fmt::Display, fs::read_to_string, io::Write, path::PathBuf};

use fmt::Config;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use toml_edit::{DocumentMut, Item, Table};

mod fmt;
mod sort;
#[cfg(test)]
mod test_utils;

const EXTRA_HELP: &str = "\
    NOTE: formatting is applied after the check for sorting so \
          sorted but unformatted toml will not cause a failure";

type IoResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Ensure Cargo.toml dependency tables are sorted.
#[derive(clap::Parser, Debug)]
#[command(author, version, bin_name = "cargo sort", after_help = EXTRA_HELP)]
pub struct Cli {
    /// sets cwd, must contain a Cargo.toml file
    #[arg(value_name = "CWD")]
    pub cwd: Vec<String>,

    /// Returns non-zero exit code if Cargo.toml is unsorted, overrides default behavior
    #[arg(short, long)]
    pub check: bool,

    /// Prints Cargo.toml, lexically sorted, to stdout
    #[arg(short, long, conflicts_with = "check")]
    pub print: bool,

    /// Skips formatting after sorting
    #[arg(short = 'n', long)]
    pub no_format: bool,

    /// Also returns non-zero exit code if formatting changes
    #[arg(long, requires = "check")]
    pub check_format: bool,

    /// Checks every crate in a workspace
    #[arg(short, long)]
    pub workspace: bool,

    /// Keep blank lines when sorting groups of key value pairs
    #[arg(short, long)]
    pub grouped: bool,

    /// List the order tables should be written out
    /// (--order package,dependencies,features)
    #[arg(short, long, value_delimiter = ',')]
    pub order: Vec<String>,

    /// List of workspace members to ignore while checking/formatting. Supports glob patterns `*`
    /// and `?`.
    /// (--ignore member_to_ignore,"ignore*")
    #[arg(short, long, requires = "workspace", value_delimiter = ',')]
    pub ignore: Vec<String>,
}

fn write_red<S: Display>(highlight: &str, msg: S) -> IoResult<()> {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
    write!(stderr, "{highlight}")?;
    stderr.reset()?;
    writeln!(stderr, "{msg}").map_err(Into::into)
}

fn write_green<S: Display>(highlight: &str, msg: S) -> IoResult<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
    write!(stdout, "{highlight}")?;
    stdout.reset()?;
    writeln!(stdout, "{msg}").map_err(Into::into)
}

fn check_toml(path: &str, cli: &Cli, config: &Config) -> IoResult<bool> {
    let mut path = PathBuf::from(path);
    if path.extension().is_none() {
        path.push("Cargo.toml");
    }

    let krate = path.components().nth_back(1).ok_or("No crate folder found")?.as_os_str();

    write_green("Checking ", format!("{}...", krate.to_string_lossy()))?;

    let toml_raw = read_to_string(&path)
        .map_err(|_| format!("No file found at: {}", path.display()))?;

    let crlf = toml_raw.contains("\r\n");

    let mut config = config.clone();
    if config.crlf.is_none() {
        config.crlf = Some(crlf);
    }

    let mut sorted =
        sort::sort_toml(&toml_raw, sort::MATCHER, cli.grouped, &config.table_order);
    let mut sorted_str = sorted.to_string();

    let is_formatted =
        // if no-format is not found apply formatting
        if !cli.no_format || cli.check_format {
            let original = sorted_str.clone();
            fmt::fmt_toml(&mut sorted, &config);
            sorted_str = sorted.to_string();
            original == sorted_str
        } else {
            true
        };

    if config.crlf.unwrap_or(fmt::DEF_CRLF) && !sorted_str.contains("\r\n") {
        sorted_str = sorted_str.replace('\n', "\r\n");
    }

    if cli.print {
        print!("{sorted_str}");
        return Ok(true);
    }

    let is_sorted = toml_raw == sorted_str;
    if cli.check {
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

    if !is_sorted {
        std::fs::write(&path, &sorted_str)?;
        write_green(
            "Finished: ",
            format!("Cargo.toml for {:?} has been rewritten", krate.to_string_lossy()),
        )?;
    } else {
        write_green(
            "Finished: ",
            format!(
                "Cargo.toml for {} is sorted already, no changes made",
                krate.to_string_lossy()
            ),
        )?;
    }

    Ok(true)
}

/// Expand workspace member definition, if it contains the `*` or `?` glob patterns. If a pattern
/// is present, use it to glob the provided `dir` and return all subdirectories that match the
/// pattern. If the member definition does not contain a pattern a vec containing `<dir>/member` is
/// returned.
/// Returns an error if the `member` contains a pattern that could not be used in the [`glob::glob`].
fn parse_workspace_member(member: &str, dir: &str) -> IoResult<Vec<String>> {
    let base_path = format!("{dir}/{member}");
    if member.contains('*') || member.contains('?') {
        let parsed_members = glob::glob(&base_path)?
            .filter_map(|globbed_path| match globbed_path {
                Ok(path) if path.is_dir() => Some(path.display().to_string()),
                _ => None,
            })
            .collect();

        Ok(parsed_members)
    } else {
        Ok(vec![base_path])
    }
}

/// Reads the workspace members, expands any wild cards and excludes members that should not be
/// processed by `cargo sort`. All members that are either in the `exclude` section of the
/// workspace section or that are specified in the `ignore` parameter will be removed.
/// Returns an error if [`parse_workspace_member`] failed to parse a glob pattern in any workspace
/// member.
fn parse_and_filter_workspace_members(
    ws: &Table,
    dir: &str,
    ignore: &[String],
) -> IoResult<Vec<String>> {
    // The workspace excludes, used to filter members by
    let mut excludes: Vec<&str> =
        ws.get("exclude").map_or_else(Vec::new, array_string_members);
    let members_to_ignore = ignore
        .iter()
        .map(|to_ignore| parse_workspace_member(to_ignore, dir))
        .collect::<IoResult<Vec<_>>>()?;
    excludes
        .extend(members_to_ignore.iter().flatten().map(|to_ignore| to_ignore.as_str()));

    let members: Vec<String> = ws
        .get("members")
        .map_or_else(Vec::new, array_string_members)
        .iter()
        .map(|member| parse_workspace_member(member, dir))
        .collect::<IoResult<Vec<_>>>()?
        .into_iter()
        .flatten()
        .filter(|parsed_member| {
            !excludes.iter().any(|excl| parsed_member.ends_with(excl))
        })
        .collect();

    Ok(members)
}

fn _main() -> IoResult<()> {
    let mut args: Vec<String> = std::env::args().collect();
    // remove "sort" when invoked `cargo sort` sort is the first arg
    // https://github.com/rust-lang/cargo/issues/7653
    if args.len() > 1 && args[1] == "sort" {
        args.remove(1);
    }
    let cli = <Cli as clap::Parser>::parse_from(args);

    let cwd = std::env::current_dir()
        .map_err(|e| format!("no current directory found: {e}"))?;
    let dir = cwd.to_string_lossy();

    let mut filtered_matches: Vec<String> = cli.cwd.clone();
    let is_posible_workspace = filtered_matches.is_empty() || filtered_matches.len() == 1;
    if filtered_matches.is_empty() {
        filtered_matches.push(dir.to_string());
    }

    if cli.workspace && is_posible_workspace {
        let dir = filtered_matches[0].to_string();
        let mut path = PathBuf::from(&dir);
        if path.extension().is_none() {
            path.push("Cargo.toml");
        }

        let raw_toml = read_to_string(&path)
            .map_err(|_| format!("no file found at: {}", path.display()))?;

        let toml = raw_toml.parse::<DocumentMut>()?;
        let workspace = toml.get("workspace");
        if let Some(Item::Table(ws)) = workspace {
            let members =
                parse_and_filter_workspace_members(ws, &dir, cli.ignore.as_slice())
                    // NOTE:: The `parse_and_filter_workspace_members` currently only returns errors
                    // from `glob::glob` (via `parse_workspace_member`) so it is okay to print the glob
                    // error here. Should this change, the content or placement of the error message (printing)
                    // needs to be updated.
                    .unwrap_or_else(|e| {
                        write_red("error: ", format!("Glob failed: {e}")).unwrap();
                        std::process::exit(1);
                    });

            filtered_matches.extend(members);
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

    if !cli.order.is_empty() {
        config.table_order = cli.order.clone();
    }

    let mut flag = true;
    for sorted in filtered_matches.iter().map(|path| check_toml(path, &cli, &config)) {
        if !(sorted?) {
            flag = false;
        }
    }

    if !flag {
        return Err("Some Cargo.toml files are not sorted or formatted".into());
    }
    Ok(())
}

fn array_string_members(value: &Item) -> Vec<&str> {
    value.as_array().into_iter().flatten().filter_map(|s| s.as_str()).collect()
}

fn main() {
    _main().unwrap_or_else(|e| {
        write_red("error: ", e).unwrap();
        std::process::exit(1);
    });
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
//         s.parse::<DocumentMut>().unwrap();
//     }
// }
//
#[cfg(test)]
mod test {
    use std::fs::read_to_string;
    use std::path::Path;

    use toml_edit::{DocumentMut, Item};

    use crate::{parse_and_filter_workspace_members, parse_workspace_member};

    #[test]
    fn member_name_expansion_without_wildcard() {
        similar_asserts::assert_eq!(
            parse_workspace_member("mock_data", "mock_workspace").unwrap(),
            vec!["mock_workspace/mock_data".to_owned()],
        );
    }

    #[test]
    fn member_name_expansion_question_mark() {
        similar_asserts::assert_eq!(
            parse_workspace_member("m?ck_data", "mock_workspace").unwrap(),
            vec!["mock_workspace/mock_data".to_owned()],
        );
    }

    #[test]
    fn member_name_expansion_star() {
        similar_asserts::assert_eq!(
            parse_workspace_member("mock*", "mock_workspace").unwrap(),
            vec![
                "mock_workspace/mock_cli".to_owned(),
                "mock_workspace/mock_data".to_owned()
            ],
        );
    }

    #[test]
    fn parse_workspace_without_ignores() {
        let raw_toml = read_to_string(Path::new("mock_workspace/Cargo.toml"))
            .expect("No file found at: mock_workspace/Cargo.toml");

        let toml = raw_toml.parse::<DocumentMut>().expect("Failed to parse raw_toml.");

        if let Some(Item::Table(workspace)) = toml.get("workspace") {
            let workspace_members =
                parse_and_filter_workspace_members(workspace, "mock_workspace", &[])
                    .expect("Failed to parse workspace members.");

            similar_asserts::assert_eq!(
                workspace_members,
                vec![
                    "mock_workspace/mock_cli".to_owned(),
                    "mock_workspace/mock_data".to_owned()
                ],
            );
        } else {
            panic!("Failed to get workspace from TOML file")
        }
    }

    #[test]
    fn parse_workspace_with_ignores() {
        let raw_toml = read_to_string(Path::new("mock_workspace/Cargo.toml"))
            .expect("No file found at: mock_workspace/Cargo.toml");

        let toml = raw_toml.parse::<DocumentMut>().expect("Failed to parse raw_toml.");

        if let Some(Item::Table(workspace)) = toml.get("workspace") {
            let workspace_members = parse_and_filter_workspace_members(
                workspace,
                "mock_workspace",
                &["mock_data".to_owned()],
            )
            .expect("Failed to parse workspace members.");

            similar_asserts::assert_eq!(
                workspace_members,
                vec!["mock_workspace/mock_cli".to_owned()],
            );
        } else {
            panic!("Failed to get workspace from TOML file")
        }
    }

    #[test]
    fn parse_workspace_with_wildcard_ignores() {
        let raw_toml = read_to_string(Path::new("mock_workspace/Cargo.toml"))
            .expect("No file found at: mock_workspace/Cargo.toml");

        let toml = raw_toml.parse::<DocumentMut>().expect("Failed to parse raw_toml.");

        if let Some(Item::Table(workspace)) = toml.get("workspace") {
            let workspace_members = parse_and_filter_workspace_members(
                workspace,
                "mock_workspace",
                &["mock_c??".to_owned()],
            )
            .expect("Failed to parse workspace members.");

            similar_asserts::assert_eq!(
                workspace_members,
                vec!["mock_workspace/mock_data".to_owned()],
            );
        } else {
            panic!("Failed to get workspace from TOML file")
        }
    }
}
