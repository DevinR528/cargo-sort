#![allow(dead_code)]

use std::collections::VecDeque;
use std::result::Result;

use super::err::{ ParseTomlError, TomlErrorKind };
use super::parse::Parse;
use super::toml_ty::{ TomlHeader, TomlItems, };

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
        Self { chunks: VecDeque::from(v) }
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
        if line.starts_with("[") {
            let header = self.chunks.pop_front().unwrap();

            let t_header = TomlHeader::parse(header)?;
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
                Some(l) => l.trim(),
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

// impl<'p> Parse<String> for TomlString {
//     type Return = TomlHeader;
//     type Error = ParseTomlError;

//     fn parse(header: String) -> Result<Self::Return, Self::Error> {
//         if header.contains(".") {
//             let segmented = header.trim_matches(|c| c == '[' || c == ']');
//             let seg = segmented.split(".").map(|s| s.to_owned()).collect();
//             // println!("SEG {:#?}", seg);
//             return Ok(TomlHeader {
//                 inner: header.into(),
//                 seg: seg,
//                 extended: true,
//             });
//         }
//         let seg: Vec<String> = header
//             .trim_matches(|c| c == '[' || c == ']')
//             .split(".")
//             .map(Into::into)
//             .collect();

//         // println!("SEG {:#?}", seg);

//         if seg.is_empty() {
//             let span = header.to_string();
//             return Err(ParseTomlError::new(
//                 "No value inside header".into(),
//                 TomlErrorKind::UnexpectedToken(span),
//             ));
//         }
//         Ok(TomlHeader {
//             inner: header.into(),
//             seg,
//             extended: false,
//         })
//     }
// }

// impl<'p> Parse<Vec<String>> for TomlString {
//     type Return = TomlItems;
//     type Error = ParseTomlError;

//     fn parse(lines: Vec<String>) -> Result<Self::Return, Self::Error> {
//         let toml_items = TomlItems::new(lines);
//         Ok(toml_items)
//     }
// }