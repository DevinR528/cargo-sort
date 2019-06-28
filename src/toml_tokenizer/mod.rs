use std::collections::VecDeque;
use std::io;
use std::result::Result;

mod err;
use err::{ParseTomlError, TomlErrorKind};
mod toml_ty;
use toml_ty::{TomlHeader, TomlItems, TomlTable};

#[cfg(windows)]
const EOL: &'static str = "\r\n";
#[cfg(not(windows))]
const EOL: &'static str = "\n";

#[derive(Debug, Clone)]
struct TomlString {
    chunks: VecDeque<String>,
}

impl TomlString {
    fn default() -> Self {
        TomlString {
            chunks: VecDeque::default(),
        }
    }

    fn has_more(&self) -> bool {
        println!("TOML S {:#?}", self.chunks);
        if let Some(c) = self.chunks.front() {
            if c.len() > 0 {
                match self.chunks.front() {
                    Some(line) => line.contains("["),
                    None => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    pub(super) fn parse_header(&mut self) -> Result<TomlHeader, ParseTomlError> {
        let line = match self.chunks.iter().next() {
            Some(l) => l,
            None => {
                // this should not happen
                return Err(ParseTomlError::new(
                    "Found empty .toml file".into(),
                    TomlErrorKind::UnexpectedToken("".into()),
                ));
            }
        };
        //for line in self.chunks {
        if line.starts_with("[") {
            let header = self.chunks.pop_front().unwrap();
            println!("{}", header);

            let t_header = TomlString::parse(header).unwrap();
            Ok(t_header)
        } else {
            Err(ParseTomlError::new(
                "Header did not start with [".into(),
                TomlErrorKind::UnexpectedToken(line.to_owned()),
            ))
        }
    }

    pub(super) fn parse_itmes(&mut self) -> Result<TomlItems, ParseTomlError> {
        let mut items = Vec::default();
        let mut end = false;
        loop {
            let line = match self.chunks.iter().next() {
                Some(l) => l,
                None => {
                    end = true;
                    ""
                }
            };

            if line.is_empty() || line.starts_with("\r") {
                if !end {
                    self.chunks.pop_front().unwrap();
                }
                // println!("{:#?}", items);
                let t_items = TomlString::parse(items)?;
                return Ok(t_items);
            } else {
                let item = self.chunks.pop_front().unwrap();
                println!("{}", item);
                items.push(item);
            }
        }
        println!("ITEMS NEVER {:#?}", self.chunks);
    }
}

trait Parse<T> {
    type Return;
    type Error;
    fn parse(s: T) -> Result<Self::Return, Self::Error>;
}

impl<'p> Parse<String> for TomlString {
    type Return = TomlHeader;
    type Error = ParseTomlError;

    fn parse(header: String) -> Result<Self::Return, Self::Error> {
        if header.contains(".") {
            let segmented = header.trim_matches(|c| c == '[' || c == ']');
            let seg = segmented.split(".").map(|s| s.to_owned()).collect();
            // println!("SEG {:#?}", seg);
            return Ok(TomlHeader {
                inner: header.into(),
                seg: seg,
                extended: true,
            });
        }
        let seg: Vec<String> = header
            .trim_matches(|c| c == '[' || c == ']')
            .split(".")
            .map(Into::into)
            .collect();

        // println!("SEG {:#?}", seg);

        if seg.is_empty() {
            let span = header.to_string();
            return Err(ParseTomlError::new(
                "No value inside header".into(),
                TomlErrorKind::UnexpectedToken(span),
            ));
        }
        Ok(TomlHeader {
            inner: header.into(),
            seg,
            extended: false,
        })
    }
}

impl<'p> Parse<Vec<String>> for TomlString {
    type Return = TomlItems;
    type Error = ParseTomlError;

    fn parse(lines: Vec<String>) -> Result<Self::Return, Self::Error> {
        println!("IN ITEMS {:#?}", lines);
        let toml_items = TomlItems::new(lines);
        Ok(toml_items)
    }
}

#[derive(Debug, Clone)]
pub struct TomlTokenizer {
    pub tables: Vec<TomlTable>,
    inner: TomlString,
}

impl TomlTokenizer {
    /// Clone only the tables
    pub fn clone_tables(&self, k: &str) -> Vec<TomlTable> {
        let table: Vec<TomlTable> = self.tables.iter().map(|t| t.clone()).collect();
        table
    }

    /// Returns an owned copy of tables with headers that match key
    fn get_nested_sec(&mut self, key: &str) -> Vec<TomlTable> {
        self.tables.iter()
            .filter(|t| t.header.inner.contains(&format!("[{}.", key)))
            .map(Clone::clone)
            .collect()
    }

    /// Sorts the whole file by nested headers
    pub fn sort_nested(&mut self, fields: Vec<&str>) {

        for field in fields.iter() {
            
            let mut nested = self.get_nested_sec(field);
            // println!("UNSORTED {:#?}", nested);
            nested.sort_unstable();
            // println!("SORTED {:#?}", nested);

            match self.tables
                .windows(1)
                .position(|t| t[0].header.inner.contains(&format!("[{}.", field)))
            {
                Some(pos) => {
                    let end = nested.len() + pos;
                    nested.swap_with_slice(&mut self.tables[pos..end])
                },
                None => {}
            }

            
            //println!("SORTED {:#?}", self.tables);
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

    pub fn parse_toml(&mut self) -> Result<Self, ParseTomlError> {
        let mut new_tt = TomlTokenizer {
            tables: Vec::default(),
            inner: TomlString::default(),
        };

        while self.inner.has_more() {
            let header = match self.inner.parse_header() {
                Ok(h) => h,
                Err(e) => return Err(e),
            };

            let items = match self.inner.parse_itmes() {
                Ok(i) => i,
                Err(e) => return Err(e),
            };

            // println!("{:#?}", items);
            let table = TomlTable {
                header: header,
                items: items.clone(),
            };
            new_tt.tables.push(table);

            // println!("{:#?}", items);
        }
        Ok(new_tt)
    }

    pub fn from_str(s: &str) -> TomlTokenizer {
        // cleans input
        let temp: Vec<&str> = s.split(&format!("{}{}{}", EOL, EOL, EOL)).collect();
        let cleaned: Vec<String> = temp
            .join(&format!("{}{}", EOL, EOL))
            .lines()
            .map(|s| s.to_owned())
            .collect();
        println!("{:?}", cleaned);

        let lines_mut = VecDeque::from(cleaned);

        let content = TomlString { chunks: lines_mut };
        TomlTokenizer {
            tables: Vec::default(),
            inner: content,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_tokenizer() {
        let included_headers: Vec<&str> = vec![
            "dependencies",
            "dev-dependencies",
            "build-dependencies",
            "workspace.members",
            "workspace.exclude",
        ];

        let f = std::fs::read_to_string("examp/right.toml").expect("no file found");
        //println!("{}", f);
        let mut tt = TomlTokenizer::from_str(&f).parse_toml().unwrap();
        //println!("{:#?}", tt);
        let unsorted = tt.clone();
        tt.sort_nested(included_headers);
        //println!("{:#?}", tt);
        assert_ne!(unsorted, tt)
    }
}
