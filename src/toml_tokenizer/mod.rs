#![allow(dead_code)]

use std::collections::VecDeque;
use std::result::Result;

mod err;
use err::{ParseTomlError, TomlErrorKind};
pub mod parse;
use parse::Parse;
mod toml_ty;
use toml_ty::{TomlHeader, TomlItems, TomlTable};
mod toml_str;
use toml_str::TomlString;

#[cfg(windows)]
const EOL: &'static str = "\r\n";
#[cfg(not(windows))]
const EOL: &'static str = "\n";


#[derive(Debug, Clone)]
pub struct TomlTokenizer {
    pub tables: Vec<TomlTable>,
    inner: TomlString,
}

/// Toml Tokenizer 
impl TomlTokenizer {

    fn new() -> Self {
        Self {
            tables: Vec::default(),
            inner: TomlString::default(),
        }
    }

    /// Clone only the tables
    pub fn clone_tables(&self) -> Vec<TomlTable> {
        self.tables.clone()
    }

    // TODO Remove when drain_filter is stable
    /// Destructivly removes and returns elements from vec
    /// based on P: predicate.
    fn drain_filter<P>(&mut self, pred: P) -> FilterTake<'_, P>
    where
        P: Fn(&TomlTable) -> bool
    {
        FilterTake::new(self, pred)
    }

    /// Returns taken tables from tokenizer with headers that match key
    /// filter_take removes items from self 
    /// 
    /// # Arguments
    /// * `key`: compared with .contains and formatted "[{key}."
    // this allows for:
    // [deps.foo]
    // a="0"
    // a="0"
    // 
    // [other.thing]
    // b=""
    // 
    // [deps.bar]
    // a=""
    // will now be grouped (starting at deps.foo) and sorted deps.bar then deps.foo
    fn take_nested_sel(&mut self, key: &str) -> (usize, Vec<TomlTable>) {
        self.drain_filter(|t| {
            t.header.inner.contains(&format!("[{}.", key))
        }).iter_with_pos()
        .collect()
    }

    /// Sorts the whole file by nested headers
    pub fn sort_nested(&mut self, field: &str) {

        let (start, mut nested) = self.take_nested_sel(field);
            // println!("UNSORTED {:#?}", nested);
            nested.sort_unstable();

            // println!("PRE {}:  {:#?}", field, nested);
            nested.reverse();
            for table in nested {
                self.tables.insert(start, table);
            }
    }

    pub fn sort_items(&mut self, key: &str) {
        let (start, mut tables) = self.drain_filter(|t| {
            t.header.inner == format!("[{}]", key)
        }).iter_with_pos().collect();

        tables.iter_mut().for_each(|t| {
            t.items.items.sort_unstable();
            println!("IN FOREACH{:#?}", t.items.items);
        });

        tables.reverse();
        for table in tables {
            self.tables.insert(start, table);
        }
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

    // pub fn parse_toml(&mut self) -> Result<Self, ParseTomlError> {
    //     let mut new_tt = TomlTokenizer {
    //         tables: Vec::default(),
    //         inner: TomlString::default(),
    //     };

    //     while self.inner.has_more() {
    //         let header = match self.inner.parse_header() {
    //             Ok(h) => h,
    //             Err(e) => return Err(e),
    //         };

    //         let items = match self.inner.parse_itmes() {
    //             Ok(i) => i,
    //             Err(e) => return Err(e),
    //         };

    //         // println!("{:#?}", items);
    //         let table = TomlTable {
    //             header: header,
    //             items: items.clone(),
    //         };
    //         new_tt.tables.push(table);

    //         // println!("{:#?}", items);
    //     }
    //     Ok(new_tt)
    // }

    // pub fn from_str(s: &str) -> TomlTokenizer {
    //     // cleans input
    //     let temp: Vec<&str> = s.split(&format!("{}{}{}", EOL, EOL, EOL)).collect();
    //     let cleaned: Vec<String> = temp
    //         .join(&format!("{}{}", EOL, EOL))
    //         .lines()
    //         // mostly for tests, removes whitespace from lines
    //         .map(|s| s.trim().to_owned())
    //         .collect();

    //     let lines = VecDeque::from(cleaned);
    //     let content = TomlString::new(lines);
    //     TomlTokenizer {
    //         tables: Vec::default(),
    //         inner: content,
    //     }
    // }
}

impl Parse<&str> for TomlTokenizer {

    type Item = TomlTokenizer;
    type Error = ParseTomlError;

    fn parse(s: &str) -> Result<Self::Item, Self::Error> {
        // cleans input
        let temp: Vec<&str> = s.split(&format!("{}{}{}", EOL, EOL, EOL)).collect();
        let cleaned: Vec<String> = temp
            .join(&format!("{}{}", EOL, EOL))
            .lines()
            // mostly for tests, removes whitespace from lines
            .map(|s| s.trim().to_owned())
            .collect();

        let mut tokenizer = TomlTokenizer {
            tables: Vec::default(),
            inner: TomlString::from(cleaned),
        };

        while tokenizer.inner.has_more() {
            let header = match tokenizer.inner.parse_header() {
                Ok(h) => h,
                Err(e) => return Err(e),
            };

            let items = match tokenizer.inner.parse_itmes() {
                Ok(i) => i,
                Err(e) => return Err(e),
            };
            
            let table = TomlTable { header, items, };
            tokenizer.tables.push(table);
            // println!("{:#?}", items);
        }
        Ok(tokenizer)
    }
}

impl PartialEq for TomlTokenizer {
    fn eq(&self, other: &TomlTokenizer) -> bool {
        let mut flag = true;
        for (i, table) in self.tables.iter().enumerate() {
            flag = table == &other.tables[i];
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
        // println!("{:#?}", tokens.tables);
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

    fn iter_with_pos(mut self) -> Self
    where 
        P: Fn(&TomlTable) -> bool 
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

    #[test]
    fn parse_sort_ne() {
        let included_headers: Vec<&str> = vec![
            "dependencies",
            "dev-dependencies",
            "build-dependencies",
            "workspace.members",
            "workspace.exclude",
        ];

        let f = std::fs::read_to_string("examp/right.toml").expect("no file found");
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
        let mut toml = r#"[dependencies]
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
        println!("{:#?}", tt);
        // we get to test this too
        let (pos, taken) = tt.drain_filter(|table| {
            table.header.inner == "[foo]"
        }).iter_with_pos().collect();

        assert_eq!(taken.len(), 1);
        assert_eq!(pos, 2);

    }

    #[test]
    fn sort_items() {
        let mut toml = r#"[dependencies]
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

        let sorted = vec![
            r#"a="0""#,
            r#"c="0""#,
            r#"f="0""#,
        ];

        let mut tt = TomlTokenizer::parse(toml).unwrap();
        //println!("{:#?}", tt);
        // we get to test this too
        let control = tt.clone_tables();
        tt.sort_items("dev-dependencies");
        assert_ne!(tt.tables[1], control[1]);
        assert_eq!(tt.tables[1].items.items, sorted);
    }

    #[test]
    fn sort_ungrouped() {
        let mut toml = r#"[dependencies.syn]
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

        let sorted = vec![
r#"[dependencies.alpha]
version = "0.15"
default-features = false
features = ["full", "parsing", "printing", "visit-mut"]"#,

r#"[dependencies.beta]
version = "0.15"
default-features = false
features = ["full", "parsing", "printing", "visit-mut"]"#,

r#"[dependencies.syn]
version = "0.15"
default-features = false
features = ["full", "parsing", "printing", "visit-mut"]"#,
        ];

        let mut tt = TomlTokenizer::parse(toml).unwrap();
        //println!("{:#?}", tt);
        let control = tt.clone_tables();
        tt.sort_nested("dependencies");
        println!("{:#?}", tt.tables);
        assert_ne!(tt.tables[1], control[1]);
        //assert_eq!(tt.tables[1].items.items, sorted);
    }

}
