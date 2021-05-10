use std::str::FromStr;

use crate::toml_edit::{Document, Item, Table, Value};

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
}

impl Default for Config {
    fn default() -> Self {
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
        }
    }
}

impl Config {
    // Used in testing and fuzzing
    #[allow(dead_code)]
    pub(crate) const fn new() -> Self {
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
        })
    }
}

fn fmt_value(value: &mut Value, config: &Config) {
    match value {
        Value::Array(arr) => {
            arr.trailing_comma = config.always_trailing_comma;
            arr.fmt(config.compact_arrays, config.multiline_trailing_comma);
        }
        Value::InlineTable(table) => {
            table.fmt(config.compact_inline_tables);
        }
        // Since the above variants have fmt methods we can only ever
        // get here from a headed table (`[header] key = val`)
        val => {
            if config.space_around_eq && val.decor().prefix().is_empty() {
                val.decor_mut().prefix.push(' ');
            }
        }
    }
}

fn fmt_table(table: &mut Table, config: &Config) {
    // Checks the header decor for blank lines
    let blank_header_lines =
        table.header_decor().prefix().lines().filter(|l| !l.starts_with('#')).count();
    if config.allowed_blank_lines < blank_header_lines {
        let dec = table.header_decor_mut();
        dec.prefix = dec.prefix().replacen(
            "\n",
            "",
            blank_header_lines - config.allowed_blank_lines,
        );
    }

    for (_, item) in table.iter_mut() {
        let blank_lines =
            item.decor().prefix().lines().filter(|l| !l.starts_with('#')).count();

        // Check each item in the table for blank lines
        if config.key_value_newlines {
            if config.allowed_blank_lines < blank_lines {
                let dec = item.decor_mut();
                dec.prefix = dec.prefix().replacen(
                    "\n",
                    "",
                    blank_lines - config.allowed_blank_lines,
                );
            }
        } else {
            let dec = item.decor_mut();
            dec.prefix = if dec.prefix.contains('#') {
                dec.prefix().replacen("\n", "", blank_lines)
            } else {
                "".to_string()
            };
        }

        if config.space_around_eq && item.decor().suffix.is_empty() {
            item.decor_mut().suffix.push(' ');
        }

        match item.value_mut() {
            Item::Table(table) => {
                // stuff
                fmt_table(table, config);
            }
            Item::Value(val) => {
                fmt_value(val, config);
            }
            Item::ArrayOfTables(_) => {}
            Item::None => {}
        }
    }
}

/// Formats a toml `Document` according to `tomlfmt.toml`.
pub fn fmt_toml(toml: &mut Document, config: &Config) {
    for (_key, item) in toml.as_table_mut().iter_mut() {
        match item.value_mut() {
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
    if config.trailing_newline && !toml.to_string_in_original_order().ends_with('\n') {
        toml.trailing.push('\n');
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use super::{fmt_toml, Config, Document};

    const CONFIG: Config = Config::new();

    #[test]
    fn toml_fmt_check() {
        let input = fs::read_to_string("examp/ruma.toml").unwrap();
        let mut toml = input.parse::<Document>().unwrap();
        fmt_toml(&mut toml, &CONFIG);
        assert_ne!(input, toml.to_string_in_original_order());
        // println!("{}", toml.to_string_in_original_order());
    }

    #[test]
    fn fmt_correct() {
        let input = fs::read_to_string("examp/right.toml").unwrap();
        let mut toml = input.parse::<Document>().unwrap();
        fmt_toml(&mut toml, &CONFIG);
        assert_eq!(input, toml.to_string_in_original_order());
    }

    #[test]
    fn array() {
        let input = fs::read_to_string("examp/clippy.toml").unwrap();
        let mut toml = input.parse::<Document>().unwrap();
        fmt_toml(&mut toml, &CONFIG);
        assert_ne!(input, toml.to_string_in_original_order());
        // println!("{}", toml.to_string_in_original_order());
    }
}
