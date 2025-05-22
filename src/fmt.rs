use std::str::FromStr;

use toml_edit::{DocumentMut, Item, RawString, Table, Value};

#[cfg(target_os = "windows")]
const NEWLINE_PATTERN: &str = "\r\n";
#[cfg(not(target_os = "windows"))]
const NEWLINE_PATTERN: &str = "\n";

pub(crate) const DEF_TABLE_ORDER: &[&str] = &[
    "package",
    "workspace",
    "lib",
    "bin",
    "features",
    "dependencies",
    "build-dependencies",
    "dev-dependencies",
];

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
#[allow(dead_code)]
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

    /// Max line length before arrays are broken up with newlines.
    ///
    /// Defaults to 80.
    pub max_array_line_len: usize,

    /// Number of spaces to indent for arrays broken up with newlines.
    ///
    /// Defaults to 4.
    pub indent_count: usize,

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
            max_array_line_len: 80,
            indent_count: 4,
            space_around_eq: true,
            compact_arrays: false,
            compact_inline_tables: false,
            trailing_newline: true,
            key_value_newlines: true,
            allowed_blank_lines: 1,
            crlf: false,
            table_order: DEF_TABLE_ORDER.iter().map(|s| (*s).to_owned()).collect(),
        }
    }
}

impl FromStr for Config {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(Self::new());
        }

        let toml = s.parse::<DocumentMut>().map_err(|_| "failed to parse as toml")?;
        Ok(Config {
            always_trailing_comma: toml
                .get("always_trailing_comma")
                .and_then(toml_edit::Item::as_bool)
                .unwrap_or_default(),
            multiline_trailing_comma: toml
                .get("multiline_trailing_comma")
                .and_then(toml_edit::Item::as_bool)
                .unwrap_or(true),
            max_array_line_len: toml
                .get("max_array_line_len")
                .and_then(toml_edit::Item::as_integer)
                .unwrap_or(80) as usize,
            indent_count: toml
                .get("indent_count")
                .and_then(toml_edit::Item::as_integer)
                .unwrap_or(4) as usize,
            space_around_eq: toml
                .get("space_around_eq")
                .and_then(toml_edit::Item::as_bool)
                .unwrap_or(true),
            compact_arrays: toml
                .get("compact_arrays")
                .and_then(toml_edit::Item::as_bool)
                .unwrap_or_default(),
            compact_inline_tables: toml
                .get("compact_inline_tables")
                .and_then(toml_edit::Item::as_bool)
                .unwrap_or_default(),
            trailing_newline: toml
                .get("trailing_newline")
                .and_then(toml_edit::Item::as_bool)
                .unwrap_or(true),
            key_value_newlines: toml
                .get("key_value_newlines")
                .and_then(toml_edit::Item::as_bool)
                .unwrap_or(true),
            allowed_blank_lines: toml
                .get("allowed_blank_lines")
                .and_then(toml_edit::Item::as_integer)
                .unwrap_or(1) as usize,
            crlf: toml.get("crlf").and_then(toml_edit::Item::as_bool).unwrap_or_default(),
            table_order: toml
                .get("table_order")
                .and_then(toml_edit::Item::as_array)
                .map_or(
                    DEF_TABLE_ORDER.iter().map(|s| (*s).to_owned()).collect(),
                    |arr| {
                        arr.into_iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect()
                    },
                ),
        })
    }
}

fn fmt_value(value: &mut Value, config: &Config) {
    match value {
        Value::Array(arr) => {
            if arr.to_string().len() > config.max_array_line_len {
                let old_trailing_comma = arr.trailing_comma();
                let new_trailing_comma = config.multiline_trailing_comma;

                let indent = " ".repeat(config.indent_count);
                let arr_len = arr.len();

                // Process all elements' prefix and suffix.
                for (i, val) in arr.iter_mut().enumerate() {
                    let mut prefix = val.prefix().trim().to_owned();
                    let suffix = val.suffix();

                    let mut last_suffix = None;

                    // Handle suffix: Add comma for the last element only.
                    let new_suffix =
                        if i == arr_len - 1 && old_trailing_comma != new_trailing_comma {
                            if new_trailing_comma {
                                // The last line's suffix must be cleared anyway,
                                // and append it to the prefix.
                                if !suffix.trim().is_empty() {
                                    last_suffix = Some(suffix.trim());
                                }
                                "".to_owned()
                            } else {
                                suffix.trim_end().to_owned() + NEWLINE_PATTERN
                            }
                        } else {
                            suffix.trim_end().to_owned()
                        };

                    last_suffix.map(|s| {
                        prefix.push_str(&format!("{NEWLINE_PATTERN}{s}"));
                        prefix = prefix.trim().to_owned();
                    });

                    let n_i = format!("{NEWLINE_PATTERN}{indent}");

                    // Handle prefix: Add newline and indent, preserve comments.
                    let new_prefix = if !prefix.is_empty() {
                        prefix
                            .lines()
                            .map(|line| format!("{n_i}{}", line.trim()))
                            .collect::<String>()
                            + &n_i
                    } else {
                        n_i
                    };

                    val.decor_mut().set_prefix(new_prefix);
                    val.decor_mut().set_suffix(new_suffix);
                }

                if old_trailing_comma != new_trailing_comma {
                    let trailing = arr.trailing().as_str().unwrap_or_default().trim_end();
                    if new_trailing_comma {
                        arr.set_trailing(trailing.to_owned() + NEWLINE_PATTERN);
                    } else {
                        arr.set_trailing(trailing.to_owned());
                    }
                }
                arr.set_trailing_comma(new_trailing_comma);
            } else {
                // Single-line array: Ensure no extra commas.
                // Clear suffixes first to remove input commas.
                for val in arr.iter_mut() {
                    val.decor_mut().set_suffix("");
                }
                arr.fmt();
                arr.decor_mut().set_prefix(" ");
                // Apply trailing comma to the last element if configured.
                arr.set_trailing_comma(config.always_trailing_comma);
            }
        }
        Value::InlineTable(table) => {
            table.decor_mut().set_prefix(" ");
            table.fmt();
        }
        // Since the above variants have fmt methods we can only ever
        // get here from a headed table (`[header] key = val`)
        val => {
            if config.space_around_eq
                && val
                    .decor()
                    .prefix()
                    .and_then(RawString::as_str)
                    .is_none_or(str::is_empty)
            {
                val.decor_mut().set_prefix(" ");
            }
        }
    }
}

fn fmt_table(table: &mut Table, config: &Config) {
    // Checks the header decor for blank lines

    let current_decor = table.decor().prefix().and_then(RawString::as_str).unwrap_or("");
    let mut new_decor = String::with_capacity(current_decor.len());

    let mut num_consecutive_blank_lines = 0;

    for line in current_decor.lines() {
        if line.starts_with("#") {
            new_decor.push_str(line);
            new_decor.push_str(NEWLINE_PATTERN);
            num_consecutive_blank_lines = 0;
            continue;
        }

        num_consecutive_blank_lines += 1;

        if num_consecutive_blank_lines <= config.allowed_blank_lines {
            new_decor.push_str(line);
            new_decor.push_str(NEWLINE_PATTERN);
        }
    }

    table.decor_mut().set_prefix(new_decor);

    let keys: Vec<_> = table.iter().map(|(k, _)| k.to_owned()).collect();
    for key in keys {
        let is_value_for_space = table.get(&key).is_some_and(|item| {
            item.is_value() && item.as_inline_table().is_none_or(|t| !t.is_dotted())
        });

        let mut dec = table.key_mut(&key).unwrap();
        let dec = dec.leaf_decor_mut();
        let prefix = dec.prefix().and_then(RawString::as_str).unwrap_or("");
        let blank_lines = prefix.lines().filter(|l| !l.starts_with('#')).count();

        // Check each item in the table for blank lines
        if config.key_value_newlines {
            if config.allowed_blank_lines < blank_lines {
                dec.set_prefix(prefix.replacen(
                    NEWLINE_PATTERN,
                    "",
                    blank_lines - config.allowed_blank_lines,
                ));
            }
        } else {
            dec.set_prefix(if prefix.contains('#') {
                prefix.replacen(NEWLINE_PATTERN, "", blank_lines)
            } else {
                "".to_string()
            });
        }

        // This is weirdly broken, inserts underscores into `[foo.bar]` table
        // headers. Revisit later.
        if config.space_around_eq
            && dec.suffix().and_then(RawString::as_str).is_none_or(str::is_empty)
            && is_value_for_space
        {
            dec.set_suffix(format!(
                "{}{}",
                dec.suffix().and_then(RawString::as_str).unwrap_or(""),
                ' '
            ));
        }

        match table.get_mut(&key).unwrap() {
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

/// Formats a toml `DocumentMut` according to `tomlfmt.toml`.
pub fn fmt_toml(toml: &mut DocumentMut, config: &Config) {
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
        toml.decor_mut().set_suffix("\n");
    }
}

trait ValueExt {
    fn prefix(&self) -> &str;
    fn suffix(&self) -> &str;
}

impl ValueExt for Value {
    fn prefix(&self) -> &str {
        self.decor().prefix().and_then(RawString::as_str).unwrap_or_default()
    }

    fn suffix(&self) -> &str {
        self.decor().suffix().and_then(RawString::as_str).unwrap_or_default()
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use super::{fmt_toml, Config, DocumentMut};
    use crate::test_utils::assert_eq;

    #[test]
    fn toml_fmt_check() {
        let input = fs::read_to_string("examp/ruma.toml").unwrap();
        let mut toml = input.parse::<DocumentMut>().unwrap();
        fmt_toml(&mut toml, &Config::new());
        assert_ne!(input, toml.to_string());
        // println!("{}", toml.to_string());
    }

    #[test]
    fn fmt_correct() {
        let input = fs::read_to_string("examp/right.toml").unwrap();
        let mut toml = input.parse::<DocumentMut>().unwrap();
        fmt_toml(&mut toml, &Config::new());
        assert_eq(input, toml);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn fmt_crlf_correct() {
        let input = String::from(
            "[package]\r\nname = \"priv-test\"\r\nversion = \"0.1.0\"\r\nedition = \"2021\"\r\nresolver = \"2\"\r\n\r\n# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html\r\n\r\n[dependencies]\r\nstructopt = \"0.3\"\r\n",
        );
        let expected = String::from(
            "[package]\nname = \"priv-test\"\nversion = \"0.1.0\"\nedition = \"2021\"\nresolver = \"2\"\n\n# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html\n\n[dependencies]\nstructopt = \"0.3\"\n",
        );
        let mut toml = input.parse::<DocumentMut>().unwrap();
        fmt_toml(&mut toml, &Config::new());
        similar_asserts::assert_eq!(expected, toml.to_string());
    }

    #[test]
    fn array() {
        let input = fs::read_to_string("examp/clippy.toml").unwrap();
        let expected = fs::read_to_string("examp/clippy.fmt.toml").unwrap();
        let mut toml = input.parse::<DocumentMut>().unwrap();
        fmt_toml(&mut toml, &Config::new());
        assert_eq(expected, toml);
    }

    #[test]
    fn trailing() {
        let input = fs::read_to_string("examp/trailing.toml").unwrap();
        let mut toml = input.parse::<DocumentMut>().unwrap();
        fmt_toml(&mut toml, &Config::new());
        assert_ne!(input, toml.to_string());
        // println!("{}", toml.to_string());
    }

    #[test]
    fn array_integration() {
        let input = r#"
[package]
authors = [
    "Manish Goregaokar <manishsmail@gmail.com>",
    "Andre Bogus <bogusandre@gmail.com>",
    "Oliver Schneider <clippy-iethah7aipeen8neex1a@oli-obk.de>" # Here is a comment
]
xyzabc = [
    "foo",
    "bar",
    "baz",
    ]
integration = [

    # A feature comment that makes this line very long.
    "git2",


    "tempfile", # Here is another comment.
    "abc", # Here is another comment at the end of the array.
]
"#;
        let expected = r#"
[package]
authors = [
    "Manish Goregaokar <manishsmail@gmail.com>",
    "Andre Bogus <bogusandre@gmail.com>",
    # Here is a comment
    "Oliver Schneider <clippy-iethah7aipeen8neex1a@oli-obk.de>",
]
xyzabc = ["foo", "bar", "baz"]
integration = [
    # A feature comment that makes this line very long.
    "git2",
    "tempfile",
    # Here is another comment.
    "abc", # Here is another comment at the end of the array.
]
"#;
        let mut toml = input.parse::<DocumentMut>().unwrap();
        fmt_toml(&mut toml, &Config::new());
        similar_asserts::assert_eq!(expected, toml.to_string());
    }
}
