use std::str::FromStr;

use toml_edit::{Document, Item, Table, Value};

/// The config file for formatting toml after sorting.
///
/// Use the `FromStr` to create a config from a string.
///
/// ## Example
/// ```
/// let input = "trailing_comma = true\ncrlf = true";
/// let config = input.parse::<Config>().unwrap();
/// assert!(config.trailing_comma);
/// assert!(config.crlf);
/// ```
pub struct Config {
    /// Use trailing comma where possible.
    ///
    /// Defaults to `false`.
    pub always_trailing_comma: bool,

    /// Use trailing comma for multi-line arrays.
    ///
    /// Defaults to `true`.
    pub multiline_trailing_comma: bool,

    /// Use space around equal sign for table key values.
    ///
    /// Defaults to `true`.
    pub space_around_eq: bool,

    /// Omit whitespace padding inside single-line arrays.
    ///
    /// Defaults to `false`.
    pub compact_arrays: bool,

    /// Omit whitespace padding inside inline tables.
    ///
    /// Defaults to `false`.
    pub compact_inline_tables: bool,

    /// Add trailing newline to the source.
    ///
    /// Defaults to `true`.
    pub trailing_newline: bool,

    /// Are newlines allowed between key value pairs in a table.
    ///
    /// This must be true for the `--grouped` flag to be used.
    /// Defaults to `true`.
    pub key_value_newlines: bool,

    /// The maximum amount of consecutive blank lines allowed.
    ///
    /// Defaults to `1`.
    pub allowed_blank_lines: usize,

    /// Use CRLF line endings
    ///
    /// Defaults to `false`.
    pub crlf: bool,

    /// The user specified ordering of tables in a document.
    ///
    /// All unspecified tables will come after these.
    pub table_order: Vec<String>,
}

impl Config {
    // Used in testing and fuzzing
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self {
            always_trailing_comma: false,
            multiline_trailing_comma: true,
            space_around_eq: true,
            compact_arrays: false,
            compact_inline_tables: false,
            trailing_newline: true,
            key_value_newlines: true,
            allowed_blank_lines: 1,
            crlf: false,
            table_order: [
                "package",
                "features",
                "dependencies",
                "build-dependencies",
                "dev-dependencies",
            ]
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        }
    }
}

impl FromStr for Config {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let toml = s.parse::<Document>().map_err(|_| "failed to parse as toml")?;
        Ok(Config {
            always_trailing_comma: toml["always_trailing_comma"]
                .as_bool()
                .unwrap_or_default(),
            multiline_trailing_comma: toml["multiline_trailing_comma"]
                .as_bool()
                .unwrap_or_default(),
            space_around_eq: toml["space_around_eq"].as_bool().unwrap_or(true),
            compact_arrays: toml["compact_arrays"].as_bool().unwrap_or_default(),
            compact_inline_tables: toml["compact_inline_tables"]
                .as_bool()
                .unwrap_or_default(),
            trailing_newline: toml["trailing_newline"].as_bool().unwrap_or(true),
            key_value_newlines: toml["key_value_newlines"].as_bool().unwrap_or(true),
            allowed_blank_lines: toml["allowed_blank_lines"].as_integer().unwrap_or(1)
                as usize,
            crlf: toml["crlf"].as_bool().unwrap_or_default(),
            table_order: toml["table_order"]
                .as_array()
                .into_iter()
                .flat_map(|a| a.iter())
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect(),
        })
    }
}

pub(crate) fn count_blank_lines(decor: &toml_edit::Decor) -> usize {
    decor
        .prefix()
        .map_or(Some(""), |s| s.as_str())
        .unwrap_or("")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .count()
}

fn map_as_str_or_false<F>(s: Option<&toml_edit::RawString>, f: F) -> bool
where
    F: FnOnce(&str) -> bool,
{
    s.and_then(|s| s.as_str()).map_or(false, f)
}

fn fmt_value(value: &mut Value, config: &Config) {
    match value {
        Value::Array(arr) => {
            arr.set_trailing_comma(config.always_trailing_comma);
            arr.fmt();
        }
        Value::InlineTable(table) => {
            table.fmt();
        }

        // Since the above variants have fmt methods we can only ever
        // get here from a headed table (`[header] key = val`)
        val => {
            let is_empty = map_as_str_or_false(val.decor().prefix(), str::is_empty);

            if config.space_around_eq && is_empty {
                val.decor_mut().set_prefix(" ");
            }
        }
    }
}

fn replacen(
    prefix: Option<&toml_edit::RawString>,
    pat: &str,
    to: &str,
    count: usize,
) -> String {
    prefix
        .and_then(|s| s.as_str())
        .map(|s| s.replacen(pat, to, count))
        .unwrap_or_default()
}

fn fmt_table(table: &mut Table, config: &Config) {
    #[cfg(target_os = "windows")]
    const NEWLINE_PATTERN: &'static str = "\r\n";
    #[cfg(not(target_os = "windows"))]
    const NEWLINE_PATTERN: &str = "\n";
    // Checks the header decor for blank lines
    let blank_header_lines = count_blank_lines(table.decor());

    if config.allowed_blank_lines < blank_header_lines {
        let dec = table.decor_mut();
        let prefix = replacen(
            dec.prefix(),
            NEWLINE_PATTERN,
            "",
            blank_header_lines - config.allowed_blank_lines,
        );
        dec.set_prefix(prefix);
    }

    let tab_clone = table.clone();

    for (key, item) in table.iter_mut() {
        let blank_lines = tab_clone.key_decor(&key).map_or(0, count_blank_lines);

        if let Some(decor) = match item {
            Item::Value(value) => Some(value.decor_mut()),
            Item::Table(table) => Some(table.decor_mut()),
            _ => None,
        } {
            // Check each item in the table for blank lines
            if config.key_value_newlines {
                if config.allowed_blank_lines < blank_lines {
                    let prefix = replacen(
                        decor.prefix(),
                        NEWLINE_PATTERN,
                        "",
                        blank_lines - config.allowed_blank_lines,
                    );
                    decor.set_prefix(prefix);
                }
            } else {
                let prefix = decor.prefix();

                let prefix = if map_as_str_or_false(prefix, |p| p.contains('#')) {
                    replacen(prefix, NEWLINE_PATTERN, "", blank_lines)
                } else {
                    "".to_string()
                };

                decor.set_prefix(prefix);
            }

            if config.space_around_eq && decor.suffix().is_none() {
                decor.set_suffix(" ");
            }
        }

        match item {
            Item::Table(table) => fmt_table(table, config),
            Item::Value(value) => fmt_value(value, config),
            _ => {}
        }
    }
}

/// Formats a toml `Document` according to `tomlfmt.toml`.
pub fn fmt_toml(toml: &mut Document, config: &Config) {
    for (_key, item) in toml.as_table_mut().iter_mut() {
        match item {
            Item::ArrayOfTables(table) => {
                for tab in table.iter_mut() {
                    fmt_table(tab, config);
                }
            }
            Item::Table(table) => {
                fmt_table(table, config);
            }
            Item::Value(val) => {
                fmt_value(val, config);
            }
            Item::None => {}
        }
    }

    // TODO:
    // This is TERRIBLE!! Convert the Document to a string only to check it ends with a
    // newline
    if config.trailing_newline && !toml.to_string().ends_with('\n') {
        toml.set_trailing("\n");
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use super::{fmt_toml, Config, Document};

    #[test]
    fn toml_fmt_check() {
        let input = fs::read_to_string("examp/ruma.toml").unwrap();
        let mut toml = input.parse::<Document>().unwrap();
        fmt_toml(&mut toml, &Config::new());
        assert_ne!(input, toml.to_string());
        // println!("{}", toml.to_string_in_original_order());
    }

    #[test]
    fn fmt_correct() {
        let input = fs::read_to_string("examp/right.toml").unwrap();
        let mut toml = input.parse::<Document>().unwrap();
        fmt_toml(&mut toml, &Config::new());
        #[cfg(target_os = "windows")]
        assert_eq!(input.replace("\r\n", "\n"), toml.to_string().replace("\r\n", "\n"));
        #[cfg(not(target_os = "windows"))]
        assert_eq!(input, toml.to_string());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn fmt_crlf_correct() {
        let input = String::from(
            "[package]\r\nname = \"priv-test\"\r\nversion = \"0.1.0\"\r\nedition = \"2021\"\r\nresolver = \"2\"\r\n\r\n# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html\r\n\r\n[dependencies]\r\nstructopt = \"0.3\"\r\n",
        );
        let expected = String::from(
            "[package]\nname = \"priv-test\"\nversion = \"0.1.0\"\nedition = \"2021\"\nresolver = \"2\"\n# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html\n\r\n[dependencies]\nstructopt = \"0.3\"\n",
        );
        let mut toml = input.parse::<Document>().unwrap();
        fmt_toml(&mut toml, &Config::new());
        assert_eq!(expected, toml.to_string());
    }

    #[test]
    fn array() {
        let input = fs::read_to_string("examp/clippy.toml").unwrap();
        let mut toml = input.parse::<Document>().unwrap();
        fmt_toml(&mut toml, &Config::new());
        assert_ne!(input, toml.to_string());
        // println!("{}", toml.to_string_in_original_order());
    }

    #[test]
    fn trailing() {
        let input = fs::read_to_string("examp/trailing.toml").unwrap();
        let mut toml = input.parse::<Document>().unwrap();
        fmt_toml(&mut toml, &Config::new());
        assert_ne!(input, toml.to_string());
        // println!("{}", toml.to_string_in_original_order());
    }

    #[test]
    fn to_many_newlines() {
        let input = fs::read_to_string("examp/trailing.toml").unwrap();
        let mut toml = input.parse::<Document>().unwrap();
        fmt_toml(&mut toml, &Config::new());
        assert_ne!(input, toml.to_string());
        println!("{}", input.to_string());
    }
}
