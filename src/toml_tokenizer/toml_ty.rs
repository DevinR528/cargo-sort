use super::err::{ParseTomlError, TomlErrorKind};
use super::parse::Parse;

#[derive(Debug, Clone)]
pub struct TomlKVPair {
    comment: Option<String>,
    key: Option<String>,
    val: Option<String>,
}

impl PartialEq for TomlKVPair {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.comment == other.comment
    }
}

impl Eq for TomlKVPair {}

impl PartialOrd for TomlKVPair {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TomlKVPair {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if let Some(key) = &self.key {
            if let Some(other_k) = &other.key {
                key.cmp(&other_k)
            // else we have key and comment
            } else {
                std::cmp::Ordering::Equal
            }
        } else {
            std::cmp::Ordering::Less
        }
    }
}

fn split_once(s: &str) -> (Option<String>, Option<String>) {
    let pair: Vec<&str> = s.split('=').collect();
    let mut all = pair.iter().take(pair.len());

    let first = *all.next().unwrap();

    let mut second = String::default();
    for (i, kv) in all.enumerate() {
        if i == 0 {
            second.push_str(kv);
        } else {
            second.push_str(&format!("={}", kv))
        }
    }
    (Some(first.into()), Some(second))
}

impl Parse<Vec<String>> for TomlKVPair {
    type Item = TomlItems;
    type Error = ParseTomlError;
    fn parse(lines: Vec<String>) -> Result<Self::Item, Self::Error> {
        let mut toml_items = TomlItems {
            items: Vec::default(),
            eol: "\n".into(),
        };

        let lc = lines.clone();
        let lines_clone = lc.iter();
        for (i, line) in lines_clone.enumerate() {
            if line.starts_with('#') {
                if let Some(after_comm) = lines.get(i + 1) {
                    let (key, val) = split_once(after_comm);
                    let ti = TomlKVPair {
                        key,
                        val,
                        comment: Some(line.into()),
                    };
                    toml_items.items.push(ti);
                }
            } else {
                let (key, val) = split_once(&line);
                let ti = TomlKVPair {
                    key,
                    val,
                    comment: None,
                };
                toml_items.items.push(ti);
            }
        }
        Ok(toml_items)
    }
}

#[derive(Debug, Clone)]
pub struct TomlItems {
    eol: String,
    pub items: Vec<TomlKVPair>,
}

impl<'p> Parse<Vec<String>> for TomlItems {
    type Item = TomlItems;
    type Error = ParseTomlError;

    fn parse(lines: Vec<String>) -> Result<Self::Item, Self::Error> {
        let items = TomlKVPair::parse(lines)?;
        Ok(items)
    }
}

impl PartialEq for TomlItems {
    fn eq(&self, other: &TomlItems) -> bool {
        for (i, item) in self.items.iter().enumerate() {
            if item != &other.items[i] {
                return false
            }
        }
        true
    }
}

impl Eq for TomlItems {}

impl PartialOrd for TomlItems {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TomlItems {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.items.cmp(&other.items)
    }
}

impl std::fmt::Display for TomlItems {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        for item in self.items.iter() {
            if let Some(k) = &item.key {
                if let Some(v) = &item.val {
                    write!(f, "{}={}{}", k, v, self.eol)?;
                } else {
                    write!(f, "{}{}", k, self.eol)?;
                }
            } else if let Some(com) = &item.comment {
                write!(f, "{}{}", com, self.eol)?;
            }
        }
        write!(f, "{}", self.eol)
    }
}

#[derive(Debug, Clone)]
pub struct TomlHeader {
    eol: String,
    pub extended: bool,
    pub inner: String,
    pub seg: Vec<String>,
}

impl std::fmt::Display for TomlHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{}{}", self.inner, self.eol)
    }
}

impl<'p> Parse<String> for TomlHeader {
    type Item = TomlHeader;
    type Error = ParseTomlError;

    fn parse(header: String) -> Result<Self::Item, Self::Error> {
        if header.contains('.') {
            let segmented = header.trim_matches(|c| c == '[' || c == ']');
            let seg = segmented.split('.').map(Into::into).collect();
            // println!("SEG {:#?}", seg);
            return Ok(TomlHeader {
                eol: "\n".into(),
                inner: header,
                seg,
                extended: true,
            });
        }

        // if not just a single element vec
        let seg: Vec<String> = header
            .trim_matches(|c| c == '[' || c == ']')
            .split('.')
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
            eol: "\n".into(),
            inner: header,
            seg,
            extended: false,
        })
    }
}

impl PartialEq for TomlHeader {
    fn eq(&self, other: &TomlHeader) -> bool {
        self.inner == other.inner
    }
}

#[derive(Debug, Clone)]
pub struct TomlTable {
    pub eol: String,
    pub comment: Option<String>,
    pub header: Option<TomlHeader>,
    pub items: Option<TomlItems>,
}

impl TomlTable {
    pub fn sort_items(&mut self) {
        match &mut self.items {
            Some(i) => i.items.sort(),
            None => {}
        }
    }

    pub fn set_eol(&mut self, eol: &str) {
        self.eol = eol.into();
        if let Some(h) = &mut self.header {
            h.eol = eol.into();
        }

        if let Some(i) = &mut self.items {
            i.eol = eol.into();
        }
    }
}

impl std::fmt::Display for TomlTable {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match &self {
            TomlTable {
                items: Some(items),
                header: Some(header),
                comment: Some(comm),
                ..
            } => {
                let mut cmt = comm.clone();
                while cmt.ends_with('\n') && self.eol == "\r\n" {
                    cmt.pop();
                }
                cmt.push_str(&self.eol);

                write!(f, "{}{}{}", cmt, header, items)
            }
            TomlTable {
                items: Some(items),
                header: Some(header),
                comment: None,
                ..
            } => write!(f, "{}{}", header, items),
            TomlTable {
                comment: Some(comm),
                items: None,
                header: None,
                ..
            } => {
                let mut cmt = comm.clone();
                while cmt.ends_with('\n') && self.eol == "\r\n" {
                    cmt.pop();
                }
                cmt.push_str(&self.eol);

                write!(f, "{}", cmt)
            }
            TomlTable {
                header: Some(header),
                items: None,
                comment: None,
                ..
            } => write!(f, "{}", header),
            TomlTable {
                items: Some(items),
                header: None,
                comment: None,
                ..
            } => write!(f, "{}", items),
            _ => unreachable!("should have failed to parse report bug"),
        }
    }
}

impl PartialEq for TomlTable {
    fn eq(&self, other: &TomlTable) -> bool {
        self.header == other.header && self.items == other.items
    }
}

impl Eq for TomlTable {}

impl PartialOrd for TomlTable {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TomlTable {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.header
            .as_ref()
            .unwrap()
            .inner
            .cmp(&other.header.as_ref().unwrap().inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_items_parse() {
        let item = r#"hello="{ why = yes, oh = no }""#;

        let kv = TomlKVPair::parse(item).unwrap();

        assert_eq!(kv.key.unwrap(), "hello".to_string());
        assert_eq!(kv.val.unwrap(), "\"{ why = yes, oh = no }\"".to_string());
    }
}
