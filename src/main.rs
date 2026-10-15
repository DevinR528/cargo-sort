use std::{
    collections::HashSet,
    fmt::Display,
    fs::read_to_string,
    io::Write,
    path::{Path, PathBuf},
};

use fmt::Config;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use toml_edit::{DocumentMut, Item, Table};

mod fmt;
mod sort;
#[cfg(test)]
mod test_utils;

const EXTRA_HELP: &str = "\
    NOTE: in check mode, only unsorted dependencies cause failure; \
          formatting differences are reported as warnings unless --check-format is used";

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

    /// Path to a custom config file (tomlfmt.toml)
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
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

fn write_yellow<S: Display>(highlight: &str, msg: S) -> IoResult<()> {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
    write!(stderr, "{highlight}")?;
    stderr.reset()?;
    writeln!(stderr, "{msg}").map_err(Into::into)
}

struct ProcessedToml {
    is_sorted: bool,
    is_formatted: bool,
    final_output: String,
}

fn process_toml(
    toml_raw: &str,
    grouped: bool,
    no_format: bool,
    check_format: bool,
    config: &Config,
) -> ProcessedToml {
    let mut sorted =
        sort::sort_toml(toml_raw, sort::MATCHER, grouped, &config.table_order);
    let sorted_only = sorted.to_string();
    // sort_toml/toml_edit always outputs LF, but input may have CRLF. Normalize
    // the raw input for comparison's sake only (formatting checking/fix is
    // unaffected).
    let toml_normalized = toml_raw.replace("\r\n", "\n");
    let is_sorted = toml_normalized == sorted_only;

    let (final_output, is_formatted) = if !no_format || check_format {
        fmt::fmt_toml(&mut sorted, config);
        let formatted = sorted.to_string();
        let is_fmt = sorted_only == formatted;
        (formatted, is_fmt)
    } else {
        (sorted_only, true)
    };

    let final_output =
        if config.crlf.unwrap_or(fmt::DEF_CRLF) && !final_output.contains("\r\n") {
            final_output.replace('\n', "\r\n")
        } else {
            final_output
        };

    ProcessedToml { is_sorted, is_formatted, final_output }
}

fn check_toml(path: &str, cli: &Cli, config: &Config) -> IoResult<bool> {
    let mut path = PathBuf::from(path);
    if path.is_dir() {
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

    let result =
        process_toml(&toml_raw, cli.grouped, cli.no_format, cli.check_format, &config);

    if cli.print {
        print!("{}", result.final_output);
        return Ok(true);
    }

    if cli.check {
        if !result.is_sorted {
            write_red(
                "error: ",
                format!("Dependencies for {} are not sorted", krate.to_string_lossy()),
            )?;
        }

        if !result.is_formatted {
            if cli.check_format {
                write_red(
                    "error: ",
                    format!(
                        "Cargo.toml for {} is not formatted",
                        krate.to_string_lossy()
                    ),
                )?;
            } else {
                write_yellow(
                    "warning: ",
                    format!(
                        "Cargo.toml for {} is not formatted",
                        krate.to_string_lossy()
                    ),
                )?;
            }
        }

        return Ok(result.is_sorted && (!cli.check_format || result.is_formatted));
    }

    let has_changes = toml_raw != result.final_output;
    if has_changes {
        std::fs::write(&path, &result.final_output)?;
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
    if filtered_matches.is_empty() {
        filtered_matches.push(dir.to_string());
    }

    if cli.workspace {
        filtered_matches = workspace_targets(&filtered_matches)?;
    }

    let mut config = if let Some(config_path) = &cli.config {
        read_to_string(config_path)
            .map_err(|_| format!("config file not found: {}", config_path.display()))?
            .parse::<Config>()?
    } else {
        let mut config_path = cwd.clone();
        config_path.push("tomlfmt.toml");
        read_to_string(&config_path)
            .or_else(|_err| {
                config_path.pop();
                config_path.push(".tomlfmt.toml");
                read_to_string(&config_path)
            })
            .unwrap_or_default()
            .parse::<Config>()?
    };

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

fn manifest_path(path: &str) -> PathBuf {
    let mut path = PathBuf::from(path);
    if path.is_dir() {
        path.push("Cargo.toml");
    }
    path
}

/// The paths to check with `--workspace`: every given path, followed by the members of
/// each given manifest that has a `[workspace]` table. Paths that lead to the same
/// manifest are kept only the first time they appear.
fn workspace_targets(paths: &[String]) -> IoResult<Vec<String>> {
    let mut targets = paths.to_vec();
    let mut found_workspace = false;
    for dir in paths {
        let path = manifest_path(dir);
        let raw_toml = read_to_string(&path)
            .map_err(|_| format!("no file found at: {}", path.display()))?;

        let toml = raw_toml.parse::<DocumentMut>()?;
        if let Some(Item::Table(ws)) = toml.get("workspace") {
            found_workspace = true;
            // Members are relative to the directory that holds the manifest
            let root = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            targets.extend(workspace_members(&root.display().to_string(), ws)?);
        }
    }

    if !found_workspace {
        let manifests: Vec<String> =
            paths.iter().map(|p| manifest_path(p).display().to_string()).collect();
        return Err(format!(
            "--workspace was given, but none of these manifests has a [workspace] \
             table: {}",
            manifests.join(", ")
        )
        .into());
    }

    let mut seen = HashSet::new();
    targets.retain(|target| {
        let manifest = manifest_path(target);
        seen.insert(manifest.canonicalize().unwrap_or(manifest))
    });
    Ok(targets)
}

/// The member directories of the workspace whose manifest is in `dir`.
fn workspace_members(dir: &str, ws: &Table) -> IoResult<Vec<String>> {
    let mut members = Vec::new();
    // The workspace excludes, used to filter members by
    let excludes: Vec<&str> =
        ws.get("exclude").map_or_else(Vec::new, array_string_members);
    for member in ws.get("members").map_or_else(Vec::new, array_string_members) {
        // TODO: a better test wether to glob?
        if member.contains('*') || member.contains('?') {
            'globs: for entry in glob::glob(&format!("{dir}/{member}"))
                .map_err(|e| format!("Glob failed: {e}"))?
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

                members.push(path.display().to_string());
            }
        } else {
            members.push(format!("{dir}/{member}"));
        }
    }
    Ok(members)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_config_flag_parsing() {
        let args = vec!["cargo-sort", "--config", "custom.toml"];
        let cli = <Cli as clap::Parser>::try_parse_from(args).unwrap();
        assert_eq!(cli.config, Some(PathBuf::from("custom.toml")));
    }

    #[test]
    fn cli_config_flag_with_other_flags() {
        let args = vec!["cargo-sort", "--check", "--config", "/path/to/config.toml", "."];
        let cli = <Cli as clap::Parser>::try_parse_from(args).unwrap();
        assert!(cli.check);
        assert_eq!(cli.config, Some(PathBuf::from("/path/to/config.toml")));
        assert_eq!(cli.cwd, vec!["."]);
    }

    #[test]
    fn cli_without_config_flag() {
        let args = vec!["cargo-sort", "--check", "."];
        let cli = <Cli as clap::Parser>::try_parse_from(args).unwrap();
        assert_eq!(cli.config, None);
    }

    /// A directory of `Cargo.toml` files for one test, removed when dropped.
    struct Tree(PathBuf);

    impl Tree {
        /// `manifests` are (directory, `Cargo.toml` contents) pairs.
        fn new(name: &str, manifests: &[(&str, &str)]) -> Self {
            let root = std::env::temp_dir()
                .join(format!("cargo-sort-test-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            for (dir, manifest) in manifests {
                let dir = root.join(dir);
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(dir.join("Cargo.toml"), manifest).unwrap();
            }
            Tree(root)
        }

        /// `workspace_targets` for paths relative to the tree, with the results made
        /// relative to the tree again and written with `/`.
        fn targets(&self, paths: &[&str]) -> IoResult<Vec<String>> {
            let paths: Vec<String> =
                paths.iter().map(|p| format!("{}/{p}", self.0.display())).collect();
            Ok(workspace_targets(&paths)?.iter().map(|t| self.relative(t)).collect())
        }

        fn relative(&self, path: &str) -> String {
            let rel = Path::new(path).strip_prefix(&self.0).unwrap();
            let parts: Vec<_> =
                rel.components().map(|c| c.as_os_str().to_string_lossy()).collect();
            parts.join("/")
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
    }

    const CRATE: &str = "[package]\nname = \"x\"\n";

    #[test]
    fn workspace_targets_single_root() {
        let tree = Tree::new(
            "single-root",
            &[
                ("ws", "[workspace]\nmembers = [\"a\", \"b\"]\n"),
                ("ws/a", CRATE),
                ("ws/b", CRATE),
                (
                    "ws2",
                    "[workspace]\nmembers = [\"crate-*\"]\nexclude = [\"crate-d\"]\n",
                ),
                ("ws2/crate-c", CRATE),
                ("ws2/crate-d", CRATE),
            ],
        );
        assert_eq!(tree.targets(&["ws"]).unwrap(), ["ws", "ws/a", "ws/b"]);
        assert_eq!(
            tree.targets(&["ws/Cargo.toml"]).unwrap(),
            ["ws/Cargo.toml", "ws/a", "ws/b"]
        );
        assert_eq!(tree.targets(&["ws2"]).unwrap(), ["ws2", "ws2/crate-c"]);
    }

    #[test]
    fn workspace_targets_expand_every_workspace() {
        let tree = Tree::new(
            "every-workspace",
            &[
                ("ws", "[workspace]\nmembers = [\"a\", \"b\"]\n"),
                ("ws/a", CRATE),
                ("ws/b", CRATE),
                ("ws2", "[workspace]\nmembers = [\"crates/*\"]\n"),
                ("ws2/crates/c", CRATE),
                ("ws2/crates/d", CRATE),
            ],
        );
        assert_eq!(
            tree.targets(&["ws", "ws2"]).unwrap(),
            ["ws", "ws2", "ws/a", "ws/b", "ws2/crates/c", "ws2/crates/d"]
        );
        assert_eq!(
            tree.targets(&["ws2", "ws"]).unwrap(),
            ["ws2", "ws", "ws2/crates/c", "ws2/crates/d", "ws/a", "ws/b"]
        );
    }

    #[test]
    fn workspace_targets_keep_crates_without_workspace() {
        let tree = Tree::new(
            "plain-crate",
            &[
                ("ws", "[workspace]\nmembers = [\"a\"]\n"),
                ("ws/a", CRATE),
                ("plain", CRATE),
            ],
        );
        assert_eq!(tree.targets(&["ws", "plain"]).unwrap(), ["ws", "plain", "ws/a"]);
        assert_eq!(tree.targets(&["plain", "ws"]).unwrap(), ["plain", "ws", "ws/a"]);
    }

    #[test]
    fn workspace_targets_are_deduplicated() {
        let tree = Tree::new(
            "dedupe",
            &[
                ("ws", "[workspace]\nmembers = [\"a\", \"b\"]\n"),
                ("ws/a", CRATE),
                ("ws/b", CRATE),
                ("other", "[workspace]\nmembers = [\"../ws/b\"]\n"),
            ],
        );
        // `b` is given, is a member of `ws` and is a member of `other`
        assert_eq!(
            tree.targets(&["ws", "ws/b", "other", "ws/"]).unwrap(),
            ["ws", "ws/b", "other", "ws/a"]
        );
    }

    #[test]
    fn workspace_targets_need_a_workspace() {
        let tree = Tree::new("no-workspace", &[("plain", CRATE), ("plain2", CRATE)]);
        for paths in [&["plain"][..], &["plain", "plain2"]] {
            let err = tree.targets(paths).unwrap_err().to_string();
            assert!(err.contains("--workspace") && err.contains("[workspace]"), "{err}");
        }
    }

    /// Generates small layouts from a fixed seed and checks `workspace_targets` against a
    /// model of the expected targets: every given path and every member of every given
    /// workspace, each manifest once, in the order given and then in manifest order.
    #[test]
    fn workspace_targets_generated_layouts() {
        const SEED: u64 = 139;
        const CASES: usize = 100;
        let mut state = SEED;
        let mut below = move |n: u64| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) % n
        };
        let (mut errors, mut duplicates, mut excluded, mut cross) = (0, 0, 0, 0);

        for case in 0..CASES {
            let roots = 1 + below(3) as usize;
            let mut manifests: Vec<(String, String)> = Vec::new();
            // Members of each root in manifest order, `None` for a crate without
            // `[workspace]`
            let mut members: Vec<Option<Vec<String>>> = Vec::new();
            let mut shape = Vec::new();
            for r in 0..roots {
                for m in 0..3 {
                    manifests.push((format!("r{r}/m{m}"), CRATE.to_owned()));
                }
                if below(4) == 0 {
                    manifests.push((format!("r{r}"), CRATE.to_owned()));
                    members.push(None);
                    shape.push("crate".to_owned());
                    continue;
                }
                let mut listed: Vec<String> =
                    (0..3).filter(|_| below(2) == 0).map(|m| format!("m{m}")).collect();
                let mut expanded: Vec<String> = listed.clone();
                let mut exclude = Vec::new();
                if below(2) == 0 {
                    let skip = if below(2) == 0 { Some(below(3)) } else { None };
                    for e in 0..3 {
                        manifests.push((format!("r{r}/e{e}"), CRATE.to_owned()));
                        if skip == Some(e) {
                            exclude.push(format!("\"e{e}\""));
                            excluded += 1;
                        } else {
                            expanded.push(format!("e{e}"));
                        }
                    }
                    listed.push("e*".to_owned());
                }
                if roots > 1 && below(4) == 0 {
                    let other = (r + 1 + below(roots as u64 - 1) as usize) % roots;
                    listed.push(format!("../r{other}/m0"));
                    expanded.push(format!("../r{other}/m0"));
                    cross += 1;
                }
                let quoted: Vec<String> =
                    listed.iter().map(|m| format!("\"{m}\"")).collect();
                manifests.push((
                    format!("r{r}"),
                    format!(
                        "[workspace]\nmembers = [{}]\nexclude = [{}]\n",
                        quoted.join(", "),
                        exclude.join(", ")
                    ),
                ));
                shape.push(format!("workspace{listed:?} exclude{exclude:?}"));
                members
                    .push(Some(expanded.iter().map(|m| format!("r{r}/{m}")).collect()));
            }

            let mut given: Vec<String> = (0..roots).map(|r| format!("r{r}")).collect();
            if below(3) == 0 {
                let at = below(given.len() as u64 + 1) as usize;
                given.insert(at, format!("r{}/m{}", below(roots as u64), below(3)));
            }
            if below(4) == 0 {
                given.push(format!("r{}/", below(roots as u64)));
            }

            let name =
                format!("case {case} (seed {SEED}): roots {shape:?}, given {given:?}");
            let refs: Vec<(&str, &str)> =
                manifests.iter().map(|(d, m)| (d.as_str(), m.as_str())).collect();
            let tree = Tree::new(&format!("generated-{case}"), &refs);
            let given_refs: Vec<&str> = given.iter().map(String::as_str).collect();
            let result = tree.targets(&given_refs);

            // The model: given paths, then the members of each given workspace root in
            // given order, keeping the first path for each manifest
            let normalize = |p: &str| {
                let mut parts: Vec<&str> = Vec::new();
                for part in p.split('/').filter(|s| !s.is_empty()) {
                    if part == ".." {
                        parts.pop();
                    } else {
                        parts.push(part);
                    }
                }
                parts.join("/")
            };
            let root_of = |p: &str| {
                let n = normalize(p);
                n.strip_prefix('r')
                    .and_then(|s| s.parse::<usize>().ok())
                    .map(|r| members[r].as_ref())
            };
            let mut expected: Vec<String> = given.clone();
            for g in &given {
                if let Some(Some(m)) = root_of(g) {
                    expected.extend(m.iter().cloned());
                }
            }
            let before_dedupe = expected.len();
            let mut seen = HashSet::new();
            let expected: Vec<String> =
                expected.into_iter().filter(|p| seen.insert(normalize(p))).collect();

            if members.iter().all(Option::is_none) {
                errors += 1;
                let err = result.expect_err(&name).to_string();
                assert!(err.contains("--workspace"), "{name}: {err}");
                continue;
            }
            if expected.len() < before_dedupe {
                duplicates += 1;
            }
            let targets = result.unwrap_or_else(|e| panic!("{name}: {e}"));
            let keys: Vec<String> = targets.iter().map(|t| normalize(t)).collect();
            let unique: HashSet<&String> = keys.iter().collect();
            assert_eq!(unique.len(), keys.len(), "{name}: duplicate targets {keys:?}");
            for g in &given {
                assert!(
                    keys.contains(&normalize(g)),
                    "{name}: given {g} missing: {keys:?}"
                );
            }
            for m in given.iter().filter_map(|g| root_of(g)).flatten().flatten() {
                assert!(
                    keys.contains(&normalize(m)),
                    "{name}: member {m} missing: {keys:?}"
                );
            }
            let expected_keys: Vec<String> =
                expected.iter().map(|p| normalize(p)).collect();
            assert_eq!(keys, expected_keys, "{name}: order");
        }

        // The generator has to reach every situation the model distinguishes
        assert!(errors > 0 && duplicates > 0 && excluded > 0 && cross > 0);
    }

    #[test]
    fn custom_config_loads_correctly() {
        let config_content = read_to_string("examp/custom_config.toml").unwrap();
        let config: Config = config_content.parse().unwrap();
        assert!(config.always_trailing_comma);
        assert!(!config.multiline_trailing_comma);
        assert!(!config.space_around_eq);
        assert!(config.compact_arrays);
        assert_eq!(config.indent_count, 2);
    }

    #[test]
    fn config_error_on_missing_file() {
        let result = read_to_string("nonexistent_config.toml");
        assert!(result.is_err());
    }

    #[test]
    fn check_unsorted() {
        let toml = "[dependencies]\nfoo = \"1\"\nbar = \"1\"\n";
        let config = Config::default();
        let result = process_toml(toml, false, false, false, &config);
        assert!(!result.is_sorted);
        assert!(result.is_formatted);
    }

    #[test]
    fn check_sorted_unformatted() {
        // Sorted deps, but missing space around '=' so formatting differs
        let toml = "[dependencies]\nbar=\"1\"\nfoo=\"1\"\n";
        let config = Config::default();
        let result = process_toml(toml, false, false, false, &config);
        assert!(result.is_sorted);
        assert!(!result.is_formatted);
    }

    #[test]
    fn sorted_unformatted_no_check_format() {
        let toml = "[dependencies]\nbar=\"1\"\nfoo=\"1\"\n";
        let config = Config::default();
        let result = process_toml(toml, false, false, true, &config);
        assert!(result.is_sorted);
        assert!(!result.is_formatted);
    }

    #[test]
    fn sorted_with_crlf_detected_as_sorted() {
        let toml = "[dependencies]\r\nbar = \"1\"\r\nfoo = \"1\"\r\n";
        let config = Config::default();
        let result = process_toml(toml, false, false, false, &config);
        assert!(
            result.is_sorted,
            "CRLF file with sorted deps should be detected as sorted"
        );
    }
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
