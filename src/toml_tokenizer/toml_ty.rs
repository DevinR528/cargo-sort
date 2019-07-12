use super::err::{ParseTomlError, TomlErrorKind};
use super::parse::Parse;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TomlKVPair {
    comment: Option<String>,
    key: Option<String>,
    val: Option<String>,
}

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
            std::cmp::Ordering::Equal
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

impl Parse<&str> for TomlKVPair {
    type Item = TomlKVPair;
    type Error = ParseTomlError;
    fn parse(s: &str) -> Result<Self::Item, Self::Error> {
        if s.starts_with('#') {
            Ok(TomlKVPair {
                key: None,
                val: None,
                comment: Some(s.into()),
            })
        } else {
            let (key, val) = split_once(&s);
            Ok(TomlKVPair {
                key,
                val,
                comment: None,
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TomlItems<'s> {
    pub items: Vec<TomlKVPair>,
    eol: &'s str,
}

impl<'p> Parse<Vec<String>> for TomlItems<'p> {
    type Item = TomlItems<'p>;
    type Error = ParseTomlError;

    fn parse(lines: Vec<String>) -> Result<Self::Item, Self::Error> {
        let mut toml_items = TomlItems {
            items: Vec::default(),
            eol: "\n",
        };
        for line in lines.iter() {
            let item = TomlKVPair::parse(line)?;
            toml_items.items.push(item);
        }
        Ok(toml_items)
    }
}

impl<'s> std::fmt::Display for TomlItems<'s> {
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
pub struct TomlHeader<'s> {
    pub extended: bool,
    pub inner: String,
    pub seg: Vec<String>,
    eol: &'s str,
}

impl<'s> std::fmt::Display for TomlHeader<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{}{}", self.inner, self.eol)
    }
}

impl<'s> Parse<String> for TomlHeader<'s> {
    type Item = TomlHeader<'s>;
    type Error = ParseTomlError;

    fn parse(header: String) -> Result<Self::Item, Self::Error> {
        if header.contains('.') {
            let segmented = header.trim_matches(|c| c == '[' || c == ']');
            let seg = segmented.split('.').map(Into::into).collect();
            return Ok(TomlHeader {
                inner: header,
                seg,
                extended: true,
                eol: "\n",
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
            inner: header,
            seg,
            extended: false,
            eol: "\n",
        })
    }
}

impl<'s> PartialEq for TomlHeader<'s> {
    fn eq(&self, other: &TomlHeader) -> bool {
        self.inner == other.inner
    }
}

#[derive(Debug, Clone)]
pub struct TomlTable<'t> {
    pub comment: Option<String>,
    pub header: Option<TomlHeader<'t>>,
    pub items: Option<TomlItems<'t>>,
}

impl<'s> TomlTable<'s> {

    pub fn set_eol(&mut self, eol: &str) {
        if let Some(h) = self.header {
            h.eol = eol;
        }

        if let Some(i) = self.items {
            i.eol = eol;
        }
    }

    pub fn sort_items(&mut self) {
        match &mut self.items {
            Some(i) => i.items.sort(),
            None => {}
        }
    }
}

impl<'s> std::fmt::Display for TomlTable<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            TomlTable {
                items: Some(items),
                header: Some(header),
                comment: Some(comment),
            } => write!(f, "{}{}{}", comment, header, items,),
            TomlTable {
                items: Some(items),
                header: Some(header),
                ..
            } => write!(f, "{}{}", header, items),
            TomlTable {
                comment: Some(comment),
                ..
            } => write!(f, "{}", comment),
            TomlTable {
                header: Some(header),
                ..
            } => write!(f, "{}", header),
            TomlTable {
                items: Some(items), ..
            } => write!(f, "{}", items),
            TomlTable {
                comment: None,
                header: None,
                items: None,
            } => write!(f, "nothing"),
        }
    }
}

impl<'t> PartialEq for TomlTable<'t> {
    fn eq(&self, other: &TomlTable) -> bool {
        self.header == other.header && self.items == other.items
    }
}

impl<'t> Eq for TomlTable<'t> {}

impl<'t> PartialOrd for TomlTable<'t> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'t> Ord for TomlTable<'t> {
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
