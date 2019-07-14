#![allow(dead_code)]
use std::result::Result;

mod err;
use err::ParseTomlError;
pub mod parse;
use parse::Parse;
mod toml_ty;
use toml_ty::TomlTable;
mod toml_str;
use toml_str::TomlString;

#[cfg(windows)]
pub const EOL: &str = "\r\n";
#[cfg(not(windows))]
pub const EOL: &str = "\n";

#[derive(Debug, Clone)]
pub struct TomlTokenizer {
    was_sorted: bool,
    pub tables: Vec<TomlTable>,
    inner: TomlString,
}

/// Toml Tokenizer
impl TomlTokenizer {
    fn new() -> Self {
        Self {
            was_sorted: false,
            tables: Vec::default(),
            inner: TomlString::default(),
        }
    }

    /// Clone only the tables
    pub fn clone_tables(&self) -> Vec<TomlTable> {
        self.tables.clone()
    }

    pub fn set_eol(&mut self, eol: &str) {
        self.tables.iter_mut().for_each(|t| t.set_eol(eol))
    }

    // TODO Remove when drain_filter is stable
    /// Destructivly removes and returns elements from vec
    /// based on P: predicate.
    fn drain_filter<P>(&mut self, pred: P) -> FilterTake<'_, P>
    where
        P: Fn(&TomlTable) -> bool,
    {
        FilterTake::new(self, pred)
    }

    /// Returns owned tables from tokenizer with headers that match key
    /// drain_filter removes items from self
    ///
    /// # Arguments
    /// * `key`: compared with .contains() and formatted "[{key}."
    /// this allows for:
    ///
    /// [deps.foo]
    /// a="0"
    /// a="0"
    ///
    /// [other.thing]
    /// b=""
    ///
    /// [deps.bar]
    /// a=""
    /// will now be grouped (starting at deps.foo) and sorted deps.bar then deps.foo
    fn take_nested_sel(&mut self, key: &str) -> (usize, Vec<TomlTable>) {
        self.drain_filter(|t| {
            // unwrap? this would only happen if cargo.toml was empty
            if let Some(header) = &t.header {
                header.inner.contains(&format!("[{}.", key))
            } else {
                false
            }
        })
        .index_iter()
        .collect()
    }

    /// Sorts the all of .toml file by nested headers with 'key'
    pub fn sort_nested(&mut self, key: &str) {
        let (start, mut nested) = self.take_nested_sel(key);
        let unsorted = nested.clone();
        nested.sort();

        // compares vec to vec so spacing differences will not fail it
        if unsorted != nested {
            self.was_sorted = true
        }
        nested.reverse();
        for table in nested {
            self.tables.insert(start, table);
        }
    }

    /// Sorts all of the items under the header `key`
    ///
    /// # Examples
    ///
    /// ```
    /// let toml =
    /// r#"[deps]
    /// a="0"
    /// f="0"
    /// c="0"
    ///
    /// "#;
    ///
    /// let mut tt = TomlTokenizer::parse(toml).unwrap();
    /// let control = tt.clone_tables();
    ///
    /// tt.sort_items("deps");
    ///
    /// assert_ne!(tt.tables[0], control[0]);
    /// ```
    pub fn sort_items(&mut self, key: &str) {
        let (start, mut tables) = self
            .drain_filter(|t| {
                if let Some(header) = &t.header {
                    header.inner == format!("[{}]", key)
                } else {
                    false
                }
            })
            .index_iter()
            .collect();

        tables.iter_mut().for_each(|t| {
            let unsorted = t.clone();
            t.items.as_mut().unwrap().items.sort();
            // compares vec to vec so spacing differences will not fail it
            if &unsorted != t {
                self.was_sorted = true
            }
        });

        tables.reverse();
        for table in tables {
            self.tables.insert(start, table);
        }
    }

    // if true the toml file needed sorting
    pub fn was_sorted(&self) -> bool {
        self.was_sorted
    }

    pub fn iter(&self) -> TokenIter {
        TokenIter {
            inner: self,
            idx: 0,
        }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut TomlTable> {
        self.tables.iter_mut()
    }
}

impl Parse<&str> for TomlTokenizer {
    type Item = TomlTokenizer;
    type Error = ParseTomlError;

    fn parse(s: &str) -> Result<Self::Item, Self::Error> {
        let eol = if s.contains("\r\n") { "\r\n" } else { "\n" };
        // cleans input
        let temp: Vec<&str> = s.split(&format!("{}{}{}", eol, eol, eol)).collect();
        let cleaned: Vec<String> = temp
            .join(&format!("{}{}", eol, eol))
            .lines()
            // mostly for tests, removes whitespace from lines
            .map(|s| s.trim().to_owned())
            .collect();

        let mut tokenizer = TomlTokenizer {
            was_sorted: false,
            tables: Vec::default(),
            inner: TomlString::from(cleaned),
        };

        while tokenizer.inner.has_more() {
            let (comment, end) = tokenizer.inner.check_comment()?;
            if !end {
                let header = tokenizer.inner.parse_header()?;

                let items = tokenizer.inner.parse_itmes()?;

                let table = TomlTable {
                    header: Some(header),
                    items: Some(items),
                    comment,
                    eol: "\n".into(),
                };
                tokenizer.tables.push(table);
            } else {
                let table = TomlTable {
                    header: None,
                    items: None,
                    comment,
                    eol: "\n".into(),
                };
                tokenizer.tables.push(table);
            }
        }
        Ok(tokenizer)
    }
}

impl std::fmt::Display for TomlTokenizer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        for t in self.tables.iter() {
            write!(f, "{}", t)?;
        }
        Ok(())
    }
}

impl PartialEq for TomlTokenizer {
    fn eq(&self, other: &TomlTokenizer) -> bool {
        let mut flag = true;
        for (i, table) in self.tables.iter().enumerate() {
            flag = table == &other.tables[i];
            if !flag {
                return flag;
            }
        }
        flag
    }
}

impl IntoIterator for TomlTokenizer {
    type Item = TomlTable;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.tables.into_iter()
    }
}

pub struct TokenIter<'t> {
    inner: &'t TomlTokenizer,
    idx: usize,
}

impl<'t> Iterator for TokenIter<'t> {
    type Item = &'t TomlTable;

    fn next(&mut self) -> Option<Self::Item> {
        self.idx += 1;
        self.inner.tables.get(self.idx - 1)
    }
}

pub struct FilterTake<'a, P> {
    predicate: P,
    idx: usize,
    steal_idx: usize,
    old_len: usize,
    first_found_idx: usize,
    tokens: &'a mut TomlTokenizer,
    taken: Vec<TomlTable>,
}

impl<'a, P> FilterTake<'a, P> {
    pub(super) fn new(tokens: &'a mut TomlTokenizer, predicate: P) -> FilterTake<'a, P> {
        let old_len = tokens.tables.len();
        FilterTake {
            predicate,
            tokens,
            taken: Vec::default(),
            old_len,
            idx: 0,
            steal_idx: 0,
            first_found_idx: 0,
        }
    }

    fn index_iter(mut self) -> Self
    where
        P: Fn(&TomlTable) -> bool,
    {
        let mut first = true;
        while self.idx != self.old_len {
            if (self.predicate)(&mut self.tokens.tables[self.steal_idx]) {
                let val = self.tokens.tables.remove(self.steal_idx);
                self.taken.push(val);

                if first {
                    self.first_found_idx = self.steal_idx;
                    first = false;
                }
                self.idx += 1;
            } else {
                self.steal_idx += 1;
                self.idx += 1;
            }
        }
        self
    }

    fn collect(self) -> (usize, Vec<TomlTable>) {
        (self.first_found_idx, self.taken.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::{assert_eq, assert_ne};

    #[test]
    fn parse_sort_ne() {
        let included_headers: Vec<&str> = vec![
            "dependencies",
            "dev-dependencies",
            "build-dependencies",
            "workspace.members",
            "workspace.exclude",
        ];

        // any /examp/ file but right.toml thats sorted
        let f = std::fs::read_to_string("examp/fend.toml").expect("no file found");
        //println!("{}", f);
        let mut tt = TomlTokenizer::parse(&f).unwrap();
        //println!("{:#?}", tt);
        let unsorted = tt.clone_tables();
        for header in included_headers {
            tt.sort_nested(header);
        }
        assert_ne!(unsorted, tt.tables)
    }

    #[test]
    fn take_all_filter() {
        let toml = r#"[dependencies]
        a="0"
        b="0"
        c="0"

        [dev-dependencies]
        a="0"
        f="0"
        c="0"

        [foo]
        a="0"

        "#;

        let mut tt = TomlTokenizer::parse(toml).unwrap();
        let (pos, taken) = tt
            .drain_filter(|table| table.header.as_ref().unwrap().inner == "[foo]")
            .index_iter()
            .collect();

        assert_eq!(taken.len(), 1);
        assert_eq!(pos, 2);
    }

    #[test]
    fn sort_items() {
        let toml = r#"[dev-dependencies]
        a="0"
        f="0"
        c="0"

        "#;

        let sorted = r#"a="0"
c="0"
f="0"

"#;

        let mut tt = TomlTokenizer::parse(toml).unwrap();
        //println!("{:#?}", tt);
        // we get to test this too
        let control = tt.clone_tables();
        tt.sort_items("dev-dependencies");
        let tt_sort = tt.tables[0].items.as_ref().unwrap().to_string();
        assert_ne!(tt.tables[0], control[0]);
        assert_eq!(&tt_sort, sorted);
    }

    #[test]
    fn sort_items_comment() {
        let toml = r#"[dev-dependencies]
        # just to make it interesting
        a="0"
        f="0"
        c="0"

        "#;

        let sorted = r#"# just to make it interesting
a="0"
c="0"
f="0"

"#;

        let mut tt = TomlTokenizer::parse(toml).unwrap();
        //println!("{:#?}", tt);
        // we get to test this too
        let control = tt.clone_tables();
        tt.sort_items("dev-dependencies");
        let tt_sort = tt.tables[0].items.as_ref().unwrap().to_string();
        assert_ne!(tt.tables[0], control[0]);
        assert_eq!(&tt_sort, sorted);
    }

    #[test]
    fn table_comment() {
        let toml = r#"#this is a comment
        # this too

        [dev-dependencies]
        # just to make it interesting
        a = "0"
        f = "0"
        c = "0"

        #the end
        "#;

        let sorted = r#"#this is a comment
# this too

[dev-dependencies]
# just to make it interesting
a = "0"
c = "0"
f = "0"

#the end

"#;

        let mut tt = TomlTokenizer::parse(toml).unwrap();
        //println!("{:#?}", tt);
        let control = tt.clone_tables();
        tt.sort_items("dev-dependencies");
        assert_ne!(tt.tables[0], control[0]);
        assert_eq!(&tt.to_string(), sorted);
    }

    #[test]
    fn sort_ungrouped() {
        let toml = r#"[dependencies.syn]
version = "0.15"
default-features = false
features = ["full", "parsing", "printing", "visit-mut"]

[dependencies.alpha]
version = "0.15"
default-features = false
features = ["full", "parsing", "printing", "visit-mut"]

[workspace.members]
this = "that"
that = "this"

[dependencies.beta]
version = "0.15"
default-features = false
features = ["full", "parsing", "printing", "visit-mut"]

        "#;

        let sorted = r#"[dependencies.alpha]
version = "0.15"
default-features = false
features = ["full", "parsing", "printing", "visit-mut"]

[dependencies.beta]
version = "0.15"
default-features = false
features = ["full", "parsing", "printing", "visit-mut"]

[dependencies.syn]
version = "0.15"
default-features = false
features = ["full", "parsing", "printing", "visit-mut"]

[workspace.members]
this = "that"
that = "this"

"#;

        let mut tt = TomlTokenizer::parse(toml).unwrap();
        //println!("{:#?}", tt);
        let control = tt.clone_tables();
        tt.sort_nested("dependencies");
        assert_ne!(tt.tables[1], control[1]);
        assert_eq!(tt.to_string(), sorted);
    }

    #[test]
    fn test_table_display() {
        let item = r#"[foo]
a="0"

#comment

"#;

        let cmp = item.to_string().clone();
        let tt = TomlTokenizer::parse(item).unwrap();

        assert_eq!(tt.to_string(), cmp);
    }

    #[test]
    fn test_item_newline() {
        let toml = r#"[dev-dependencies]
        a="0"
        f="0"

        c="0"

        "#;

        let tt = TomlTokenizer::parse(toml);

        let err_val = ParseTomlError {
            info: "Invalid token in table".into(),
            kind: err::TomlErrorKind::UnexpectedToken("'\\n' or '\\r\\n'".to_string()),
        };

        match tt {
            Ok(_) => assert_eq!(0, 1),
            Err(e) => assert_eq!(e, err_val),
        }
    }

}
