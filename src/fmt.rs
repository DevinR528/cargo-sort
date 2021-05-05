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
    pub trailing_comma: bool,

    /// Use space around equal sign for table key values.
    pub space_around_eq: bool,

    /// Omit whitespace padding inside single-line arrays.
    pub compact_arrays: bool,

    /// Omit whitespace padding inside inline tables.
    pub compact_inline_tables: bool,

    /// Add trailing newline to the source.
    pub trailing_newline: bool,

    /// Are newlines allowed between key value pairs in a table.
    pub key_value_newlines: bool,

    /// The maximum amount of consecutive blank lines allowed.
    pub allowed_blank_lines: usize,

    /// Use CRLF line endings
    pub crlf: bool,
}

impl FromStr for Config {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let toml = s.parse::<Document>().map_err(|_| "failed to parse as toml")?;
        Ok(Config {
            trailing_comma: toml["trailing_comma"].as_bool().unwrap_or_default(),
            space_around_eq: toml["space_around_eq"].as_bool().unwrap_or(true),
            compact_arrays: toml["compact_arrays"].as_bool().unwrap_or_default(),
            compact_inline_tables: toml["compact_inline_tables"]
                .as_bool()
                .unwrap_or_default(),
            trailing_newline: toml["trailing_newline"].as_bool().unwrap_or_default(),
            key_value_newlines: toml["key_value_newlines"].as_bool().unwrap_or_default(),
            allowed_blank_lines: toml["allowed_blank_lines"].as_integer().unwrap_or(1)
                as usize,
            crlf: toml["crlf"].as_bool().unwrap_or_default(),
        })
    }
}

fn fmt_value(value: &mut Value, config: &Config) {
    match value {
        Value::Array(arr) => {
            arr.trailing_comma = config.trailing_comma;
            arr.fmt(config.compact_arrays);
        }
        Value::InlineTable(table) => {
            table.fmt(config.compact_inline_tables);
        }
        // Since the above variants have fmt methods we can only ever
        // get here from a headed table (`[header] key = val`)
        val => {
            if config.space_around_eq {
                let dec = val.decor_mut();
                dec.prefix = " ".to_string();
            }
        }
    }
}

fn fmt_table(table: &mut Table, config: &Config) {
    // Checks the header decor for blank lines
    if config.allowed_blank_lines < table.header_decor().prefix().matches('\n').count() {
        let dec = table.header_decor_mut();
        dec.prefix = "\n".repeat(config.allowed_blank_lines);
    }
    for (_, item) in table.iter_mut() {
        // Check each item in the table for blank lines
        if config.key_value_newlines {
            if config.allowed_blank_lines < item.decor().prefix().matches('\n').count() {
                let dec = item.decor_mut();
                dec.prefix = "\n".repeat(config.allowed_blank_lines);
            }
        } else {
            let dec = item.decor_mut();
            dec.prefix = "".to_string();
        }
        if config.space_around_eq {
            let dec = item.decor_mut();
            dec.suffix = " ".to_string();
        }
        match item.value_mut() {
            Item::ArrayOfTables(_) => todo!(),
            Item::Table(table) => {
                // stuff
                fmt_table(table, config);
            }
            Item::Value(val) => {
                fmt_value(val, config);
            }
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
}

#[cfg(test)]
mod test {
    use std::fs;

    use toml_edit::Document;

    use super::{fmt_toml, Config};

    const CONFIG: Config = Config {
        trailing_comma: false,
        space_around_eq: true,
        compact_arrays: false,
        compact_inline_tables: false,
        trailing_newline: true,
        key_value_newlines: true,
        allowed_blank_lines: 1,
        crlf: false,
    };

    #[test]
    fn toml_edit_check() {
        let mut input =
            fs::read_to_string("examp/ruma.toml").unwrap().parse::<Document>().unwrap();
        fmt_toml(&mut input, &CONFIG);
        println!("{}", input.to_string_in_original_order());
    }
}
