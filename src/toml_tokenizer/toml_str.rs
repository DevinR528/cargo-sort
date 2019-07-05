#![allow(dead_code)]

use std::collections::VecDeque;
use std::result::Result;

use super::err::{ParseTomlError, TomlErrorKind};
use super::parse::Parse;
use super::toml_ty::{TomlHeader, TomlItems};

#[cfg(windows)]
const EOL: &'static str = "\r\n";
#[cfg(not(windows))]
const EOL: &'static str = "\n";

#[derive(Debug, Clone)]
pub struct TomlString {
    chunks: VecDeque<String>,
}

impl TomlString {
    pub fn new(chunks: VecDeque<String>) -> Self {
        Self { chunks }
    }

    pub fn from(v: Vec<String>) -> Self {
        Self {
            chunks: VecDeque::from(v),
        }
    }

    pub fn default() -> Self {
        TomlString {
            chunks: VecDeque::default(),
        }
    }

    pub fn has_more(&self) -> bool {
        if let Some(c) = self.chunks.front() {
            if c.len() > 0 {
                match self.chunks.front() {
                    Some(line) => line.contains("[") || line.contains("#"),
                    None => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Checks for comment and also returns (Some(String), true) if EOF was reached.
    // TODO this is ugly                        here ********************
    pub(super) fn check_comment(&mut self) -> Result<(Option<String>, bool), ParseTomlError> {
        let lines_clone = self.chunks.clone();
        let mut chunk_iter = lines_clone.iter();
        let line = match chunk_iter.next() {
            Some(l) => l,
            None => {
                // this should not happen
                return Err(ParseTomlError::new(
                    "Found empty .toml file".into(),
                    TomlErrorKind::UnexpectedToken("".into()),
                ));
            }
        };

        let mut end = false;
        if line.starts_with("#") {
            let mut comment = self.chunks.pop_front().unwrap();
            comment.push_str(super::EOL);

            loop {
                let next_l = match chunk_iter.next() {
                    Some(l) => l,
                    None => {
                        end = true;
                        ""
                    }
                };

                //println!("next l: {}", next_l);
                if next_l.starts_with("#") {
                    let comm = self.chunks.pop_front().unwrap();
                    comment.push_str(&format!("{}{}", comm, super::EOL));
                } else if next_l.is_empty() && !end {
                    self.chunks.pop_front().unwrap();
                    comment.push_str(super::EOL);
                } else {
                    return Ok((Some(comment), end));
                }
            }
        }
        Ok((None, false))
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
        if line.starts_with("[") {
            let header = self.chunks.pop_front().unwrap();

            let t_header = TomlHeader::parse(header)?;
            Ok(t_header)
        } else {
            Err(ParseTomlError::new(
                "Header did not start with '['".into(),
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
                let t_items = TomlItems::parse(items)?;
                return Ok(t_items);
            } else {
                let item = self.chunks.pop_front().unwrap();
                items.push(item);
            }
        }
    }
}
