use std::str::FromStr;

use toml_edit::{Array, DocumentMut, Item, RawString, Table, Value};

#[cfg(target_os = "windows")]
pub(crate) const DEF_CRLF: bool = true;
#[cfg(not(target_os = "windows"))]
pub(crate) const DEF_CRLF: bool = false;

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

const NEWLINE_CHARS: &[char] = &['\r', '\n'];

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
#[derive(Clone)]
pub(crate) struct Config {
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
    /// Defaults to `None`, which means use the original file's line endings
    /// or the system's.
    pub crlf: Option<bool>,

    /// The user specified ordering of tables in a document.
    ///
    /// All unspecified tables will come after these.
    pub table_order: Vec<String>,

    /// Sort feature lists in dependencies.
    pub sort_feature_list: bool,
}

impl Default for Config {
    fn default() -> Self {
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
            crlf: None,
            table_order: DEF_TABLE_ORDER.iter().map(|&s| s.to_owned()).collect(),
            sort_feature_list: false,
        }
    }
}

impl FromStr for Config {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(Self::default());
        }

        let toml = s.parse::<DocumentMut>().map_err(|_| "failed to parse as toml")?;
        Ok(Config {
            always_trailing_comma: toml
                .get("always_trailing_comma")
                .and_then(Item::as_bool)
                .unwrap_or_default(),
            multiline_trailing_comma: toml
                .get("multiline_trailing_comma")
                .and_then(Item::as_bool)
                .unwrap_or(true),
            max_array_line_len: toml
                .get("max_array_line_len")
                .and_then(Item::as_integer)
                .unwrap_or(80) as usize,
            indent_count: toml.get("indent_count").and_then(Item::as_integer).unwrap_or(4)
                as usize,
            space_around_eq: toml
                .get("space_around_eq")
                .and_then(Item::as_bool)
                .unwrap_or(true),
            compact_arrays: toml
                .get("compact_arrays")
                .and_then(Item::as_bool)
                .unwrap_or_default(),
            compact_inline_tables: toml
                .get("compact_inline_tables")
                .and_then(Item::as_bool)
                .unwrap_or_default(),
            trailing_newline: toml
                .get("trailing_newline")
                .and_then(Item::as_bool)
                .unwrap_or(true),
            key_value_newlines: toml
                .get("key_value_newlines")
                .and_then(Item::as_bool)
                .unwrap_or(true),
            allowed_blank_lines: toml
                .get("allowed_blank_lines")
                .and_then(Item::as_integer)
                .unwrap_or(1) as usize,
            crlf: toml.get("crlf").and_then(Item::as_bool),
            table_order: toml.get("table_order").and_then(Item::as_array).map_or(
                DEF_TABLE_ORDER.iter().map(|&s| s.to_owned()).collect(),
                |arr| {
                    arr.into_iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_owned())
                        .collect()
                },
            ),
            sort_feature_list: toml
                .get("sort_feature_list")
                .and_then(Item::as_bool)
                .unwrap_or_default(),
        })
    }
}

#[derive(Debug)]
struct Context {
    current_path: Vec<String>,
}

impl Context {
    fn inside_dependency_section(&self) -> bool {
        match self.current_path.as_slice() {
            [section, ..]
                if (section == "dependencies"
                    || section == "dev-dependencies"
                    || section == "build-dependencies") =>
            {
                true
            }
            [workspace, dependencies, ..]
                if (workspace == "workspace" && dependencies == "dependencies") =>
            {
                true
            }
            [section, _target, key, ..]
                if (section == "target" && key == "dependencies") =>
            {
                true
            }
            _ => false,
        }
    }
}

/// Sort an array of cargo features.
///
/// Panics if a feature is not a string.
fn sort_feature_array(array: &mut Array) {
    array.sort_by_key(|v| {
        v.as_str().expect("cargo feature should be a string").to_owned()
    });
}

/// Format an array to fit on a single line.
fn format_single_line_array(array: &mut Array, config: &Config) {
    // This method formats the array in a single line with only the necessary
    // whitespaces between elements.
    array.fmt();

    // Set the trailing comma according to the config.
    array.set_trailing_comma(config.always_trailing_comma);

    // Clean up the prefix and suffix of the array.
    format_array_decor(array);
}

/// Format an array to fit on multiple lines.
fn format_multi_line_array(array: &mut Array, config: &Config) {
    let newline_pattern = if config.crlf.unwrap_or(DEF_CRLF) { "\r\n" } else { "\n" };
    let indent = " ".repeat(config.indent_count);
    let newline_and_indent = format!("{newline_pattern}{indent}");

    let array_len = array.len();
    // Comments after the comma of the last value ends up in the "trailing" parameter of
    // the array.
    let trailing_comments =
        array.trailing().as_str().unwrap_or_default().trim().to_owned();

    // First, we must enforce the formatting of comments on all elements. Since we update
    // the prefixes anyway, we set them as if we were splitting the array on multiple
    // lines because setting everything on a single line can be done easily with a single
    // method call later.
    for (i, value) in array.iter_mut().enumerate() {
        let is_last_item = i == array_len - 1;

        // For consistency we don't support comments in the suffix for elements of arrays,
        // we move them to the prefix. It allows to have the same behavior whether the
        // element has a trailing comma or not and whether the element is at the end of
        // the array or not.
        let prefix_comments = value.prefix().trim();
        let suffix_comments = value.suffix().trim();
        let trailing_comments = is_last_item.then_some(&trailing_comments);

        // Trim each line of comments to enforce the same identation and concatenate them
        // to build the new prefix.
        let mut new_prefix = prefix_comments
            .lines()
            .chain(suffix_comments.lines())
            .chain(trailing_comments.iter().flat_map(|s| s.lines()))
            .flat_map(|line| [&newline_and_indent, line.trim()])
            .collect::<String>();

        // Finally, add a newline and indentation before the element.
        new_prefix.push_str(&newline_and_indent);
        value.decor_mut().set_prefix(new_prefix);

        // Clear the suffix because we moved everything to the prefix.
        value.decor_mut().set_suffix("");
    }

    // Update the trailing comma.
    array.set_trailing_comma(config.multiline_trailing_comma);

    // Clean up the prefix and suffix of the array.
    array.set_trailing(newline_pattern);
    format_array_decor(array);
}

/// Format the prefix and suffix of an array.
fn format_array_decor(array: &mut Array) {
    let array_decor = array.decor_mut();

    // Always put a single space before the array.
    array_decor.set_prefix(" ");

    // Preserve a comment after the array but clean up the whitespaces.
    let trailing_comment = array_decor
        .suffix()
        .and_then(|trailing_comment| {
            let trailing_comment = trailing_comment.as_str()?.trim();

            // If there is a trailing comment, add a space before it.
            (!trailing_comment.is_empty()).then(|| format!(" {trailing_comment}"))
        })
        .unwrap_or_default();
    array_decor.set_suffix(trailing_comment);
}

fn fmt_value(value: &mut Value, config: &Config, ctx: &mut Context) {
    match value {
        Value::Array(array) => {
            let has_comments = array.has_comments();

            // Sorts the feature list in "expanded" representation, where each dependency
            // is in a separate section.
            let sort_features = config.sort_feature_list
                && ctx.inside_dependency_section()
                && ctx
                    .current_path
                    .last()
                    .map(|name| name == "features")
                    .unwrap_or(false);

            if has_comments {
                // If the array contains comments, we always split the array on multiple
                // lines to preserve them.
                format_multi_line_array(array, config);

                // After formatting the array, all the comments are in the prefix of their
                // element, we can sort them without risking to separate a comment from
                // its element.
                if sort_features {
                    sort_feature_array(array);
                }
            } else {
                // There are no comments, we can reorder the features right away. We must
                // do it before calling `format_single_line_array()` because
                // `Array::fmt()` removes whitespaces around the first element, so the
                // array must already be sorted.
                if sort_features {
                    sort_feature_array(array);
                }

                // If the array doesn't contain comments, we check if its length on a
                // single line would fit the current configuration. If it is too long, we
                // split it on multiple lines.
                format_single_line_array(array, config);

                if array.to_string().len() > config.max_array_line_len {
                    format_multi_line_array(array, config);
                }
            }
        }
        Value::InlineTable(table) => {
            for (key, val) in table.iter_mut() {
                if let Value::Array(array) = val {
                    let is_multi_line = array.is_multi_line();

                    // Sorts the features in inline tables.
                    let sort_features = config.sort_feature_list
                        && ctx.inside_dependency_section()
                        && key == "features";

                    // We preserve the choice of single- vs multi-line from the original
                    // manifest.
                    if is_multi_line {
                        format_multi_line_array(array, config);

                        // After formatting the array, all the comments are in the prefix
                        // of their element, we can sort them
                        // without risking to separate a comment from
                        // its element.
                        if sort_features {
                            sort_feature_array(array);
                        }
                    } else {
                        // There are no comments, we can reorder the features right away.
                        // We must do it before calling
                        // `format_single_line_array()` because
                        // `Array::fmt()` removes whitespaces around the first element, so
                        // the array must already be sorted.
                        if sort_features {
                            sort_feature_array(array);
                        }

                        format_single_line_array(array, config);
                    }
                }
            }
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

fn fmt_table(table: &mut Table, config: &Config, ctx: &mut Context) {
    let newline_pattern = if config.crlf.unwrap_or(DEF_CRLF) { "\r\n" } else { "\n" };

    // Checks the header decor for blank lines

    let current_decor = table.decor().prefix().and_then(RawString::as_str).unwrap_or("");
    let mut new_decor = String::with_capacity(current_decor.len());

    let mut num_consecutive_blank_lines = 0;

    for line in current_decor.lines() {
        if line.starts_with("#") {
            new_decor.push_str(line);
            new_decor.push_str(newline_pattern);
            num_consecutive_blank_lines = 0;
            continue;
        }

        num_consecutive_blank_lines += 1;

        if num_consecutive_blank_lines <= config.allowed_blank_lines {
            new_decor.push_str(line);
            new_decor.push_str(newline_pattern);
        }
    }

    table.decor_mut().set_prefix(new_decor);

    let keys: Vec<_> = table.iter().map(|(k, _)| k.to_owned()).collect();
    for key in keys {
        ctx.current_path.push(key.clone());
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
                    newline_pattern,
                    "",
                    blank_lines - config.allowed_blank_lines,
                ));
            }
        } else {
            dec.set_prefix(if prefix.contains('#') {
                prefix.replacen(newline_pattern, "", blank_lines)
            } else {
                "".to_owned()
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
                fmt_table(table, config, ctx);
            }
            Item::Value(val) => {
                fmt_value(val, config, ctx);
            }
            Item::ArrayOfTables(_) => {}
            Item::None => {}
        }
        ctx.current_path.pop();
    }
}

/// Formats a toml `DocumentMut` according to `tomlfmt.toml`.
pub(crate) fn fmt_toml(toml: &mut DocumentMut, config: &Config) {
    for (key, item) in toml.as_table_mut().iter_mut() {
        let mut ctx = Context { current_path: vec![key.to_string()] };
        match item {
            Item::ArrayOfTables(table) => {
                for tab in table.iter_mut() {
                    fmt_table(tab, config, &mut ctx);
                }
            }
            Item::Table(table) => {
                fmt_table(table, config, &mut ctx);
            }
            Item::Value(val) => {
                fmt_value(val, config, &mut ctx);
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
    /// The prefix of this value.
    fn prefix(&self) -> &str;

    /// The suffix of this value.
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

trait ArrayExt {
    /// Whether this array is split on multiple lines.
    fn is_multi_line(&self) -> bool;

    /// Whether this array contains comments.
    fn has_comments(&self) -> bool;
}

impl ArrayExt for Array {
    fn is_multi_line(&self) -> bool {
        self.trailing().as_str().is_some_and(|trailing| trailing.contains(NEWLINE_CHARS))
            || self.iter().any(|value| {
                value.prefix().contains(NEWLINE_CHARS)
                    || value.suffix().contains(NEWLINE_CHARS)
            })
    }

    fn has_comments(&self) -> bool {
        // The only non-whitespace characters in the prefixes and suffixes should be
        // comments.
        self.trailing().as_str().is_some_and(|trailing| !trailing.trim().is_empty())
            || self.iter().any(|value| {
                !value.prefix().trim().is_empty() || !value.suffix().trim().is_empty()
            })
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
        fmt_toml(&mut toml, &Config::default());
        assert_ne!(input, toml.to_string());
        // println!("{}", toml.to_string());
    }

    #[test]
    fn fmt_correct() {
        let input = fs::read_to_string("examp/right.toml").unwrap();
        let mut toml = input.parse::<DocumentMut>().unwrap();
        fmt_toml(&mut toml, &Config::default());
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
        fmt_toml(&mut toml, &Config::default());
        similar_asserts::assert_eq!(expected, toml.to_string());
    }

    #[test]
    fn array() {
        let input = fs::read_to_string("examp/clippy.toml").unwrap();
        let expected = fs::read_to_string("examp/clippy.fmt.toml").unwrap();
        let mut toml = input.parse::<DocumentMut>().unwrap();
        fmt_toml(&mut toml, &Config::default());
        assert_eq(expected, toml);
    }

    #[test]
    fn trailing() {
        let input = fs::read_to_string("examp/trailing.toml").unwrap();
        let mut toml = input.parse::<DocumentMut>().unwrap();
        fmt_toml(&mut toml, &Config::default());
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
    ]    # A comment after the array.
integration = [

    # A feature comment that makes this line very long.
    "git2",


    "tempfile", # Here is another comment.
    "abc", # Here is another comment at the end of the array.
]

# Test arrays in inline tables too.
[inline_tables]
unexpected_cfgs = { level = "warn", check-cfg = [
        # This comment indentation should be fixed.
    'cfg(custom_cfg_backend, values("foo"))',
    'cfg(custom_cfg_frontend, values("bar"))', # This trailing comment will be on a new line.
    'cfg(custom_cfg_flag)', # This trailing comment will be moved.
] }
# The choice of single- vs multi-line should be preserved.
include = { files = [  "*.rs",   "*.toml"]}
exclude = { files = [
        "config.rs",
    "tomledit.toml"]
}
"#;
        let expected = r#"
[package]
authors = [
    "Manish Goregaokar <manishsmail@gmail.com>",
    "Andre Bogus <bogusandre@gmail.com>",
    # Here is a comment
    "Oliver Schneider <clippy-iethah7aipeen8neex1a@oli-obk.de>",
]
xyzabc = ["foo", "bar", "baz"] # A comment after the array.
integration = [
    # A feature comment that makes this line very long.
    "git2",
    "tempfile",
    # Here is another comment.
    # Here is another comment at the end of the array.
    "abc",
]

# Test arrays in inline tables too.
[inline_tables]
unexpected_cfgs = { level = "warn", check-cfg = [
    # This comment indentation should be fixed.
    'cfg(custom_cfg_backend, values("foo"))',
    'cfg(custom_cfg_frontend, values("bar"))',
    # This trailing comment will be on a new line.
    # This trailing comment will be moved.
    'cfg(custom_cfg_flag)',
] }
# The choice of single- vs multi-line should be preserved.
include = { files = ["*.rs", "*.toml"] }
exclude = { files = [
    "config.rs",
    "tomledit.toml",
] }
"#;
        let mut toml = input.parse::<DocumentMut>().unwrap();
        fmt_toml(&mut toml, &Config::default());
        similar_asserts::assert_eq!(expected, toml.to_string());

        let expected2 = r#"
[package]
authors = [
    "Manish Goregaokar <manishsmail@gmail.com>",
    "Andre Bogus <bogusandre@gmail.com>",
    # Here is a comment
    "Oliver Schneider <clippy-iethah7aipeen8neex1a@oli-obk.de>"
]
xyzabc = ["foo", "bar", "baz"] # A comment after the array.
integration = [
    # A feature comment that makes this line very long.
    "git2",
    "tempfile",
    # Here is another comment.
    # Here is another comment at the end of the array.
    "abc"
]

# Test arrays in inline tables too.
[inline_tables]
unexpected_cfgs = { level = "warn", check-cfg = [
    # This comment indentation should be fixed.
    'cfg(custom_cfg_backend, values("foo"))',
    'cfg(custom_cfg_frontend, values("bar"))',
    # This trailing comment will be on a new line.
    # This trailing comment will be moved.
    'cfg(custom_cfg_flag)'
] }
# The choice of single- vs multi-line should be preserved.
include = { files = ["*.rs", "*.toml"] }
exclude = { files = [
    "config.rs",
    "tomledit.toml"
] }
"#;
        let mut toml = input.parse::<DocumentMut>().unwrap();
        let cfg = Config { multiline_trailing_comma: false, ..Config::default() };
        fmt_toml(&mut toml, &cfg);
        similar_asserts::assert_eq!(expected2, toml.to_string());
    }

    #[test]
    fn sort_and_format_feature_lists() {
        let config = Config {
            sort_feature_list: true,
            // 30 is chosen here so the feature list in one expanded dependency stays
            // single-line, and the other stays multi-line.
            max_array_line_len: 30,
            ..Default::default()
        };
        let input = fs::read_to_string("examp/features.toml").unwrap();
        let expected = fs::read_to_string("examp/features.sorted.toml").unwrap();

        let mut toml = input.parse::<DocumentMut>().unwrap();
        fmt_toml(&mut toml, &config);
        assert_eq(expected, toml.to_string());
    }
}
