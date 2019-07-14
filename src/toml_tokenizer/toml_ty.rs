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
        self.key.as_ref().unwrap().cmp(other.key.as_ref().unwrap())
        // if let Some(key) = &self.key {
        //     if let Some(other_k) = &other.key {
        //         key.cmp(&other_k)
        //     // else we have a parsing problem
        //     } else {
        //         unreachable!("parse error")
        //     }
        // } else {
        //     unreachable!("parse error")
        // }
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

        let mut lines = lines;
        let lc = lines.clone();
        let lines_clone = lc.iter();
        let mut i = 0;
        for line in lines_clone {
            if line.starts_with('#') {
                if let Some(after_comm) = lines.get(i + 1) {
                    let (key, val) = split_once(after_comm);
                    let ti = TomlKVPair {
                        key,
                        val,
                        comment: Some(line.to_string()),
                    };
                    toml_items.items.push(ti);
                    lines.drain(i..i + 2);
                }
            } else if let Some(line) = lines.get(i) {
                let (key, val) = split_once(line);
                let ti = TomlKVPair {
                    key,
                    val,
                    comment: None,
                };
                i += 1;
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
                return false;
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
                    if let Some(c) = &item.comment {
                        let res = match c.find('\n') {
                            Some(idx) => {
                                let (f, l) = c.split_at(idx);
                                let res = format!("{}{}", f, l.replace("\n", &self.eol));
                                res
                            }
                            None => {
                                let res = format!("{}{}", c, &self.eol);
                                res
                            }
                        };
                        write!(f, "{}{}={}{}", res, k, v, self.eol)?;
                    } else {
                        write!(f, "{}={}{}", k, v, self.eol)?;
                    }
                // this is illegal
                } else {
                    write!(f, "{}{}", k, self.eol)?;
                }
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

impl PartialOrd for TomlHeader {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TomlHeader {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl PartialEq for TomlHeader {
    fn eq(&self, other: &TomlHeader) -> bool {
        self.inner == other.inner
    }
}

impl Eq for TomlHeader {}

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
            } => write!(f, "{}{}{}", comm, header, items),
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
            } => write!(f, "{}", comm),
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
            _ => unreachable!("should have failed to parse, file a bug"),
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
        self.header.as_ref().unwrap().cmp(&other.header.as_ref().unwrap())
    }
}
