use super::err::{ ParseTomlError, TomlErrorKind };
use super::parse::Parse;
use super::toml_str::TomlString;


#[derive(Debug, Clone)]
pub struct Comment {
    inner: String,
}

#[derive(Debug, Clone)]
pub struct KVPair {
    key: String,
    val: String,
}

impl KVPair {

}

impl Parse<&str> for KVPair {
    type Item = KVPair;
    type Error = ParseTomlError;
    fn parse(s: &str) -> Result<Self::Item, Self::Error> {
        let (key, val) = split_once(&s);
        Ok(KVPair { key, val, })
    }
}

fn split_once(s: &str) -> (String, String) {
    let mut splitter = s.split('=');
    let first = splitter.next().unwrap();
    let second = splitter.fold("".to_owned(), |a, b| a + &format!(" = {}", b)); 
    (first.into(), second)
}

#[derive(Debug, Clone)]
pub struct TomlItems {
    pub items: Vec<String>,
}

impl TomlItems {
    pub fn new(items: Vec<String>) -> TomlItems {
        TomlItems { items }
    }
}

impl<'p> Parse<Vec<String>> for TomlItems {

    type Item = TomlItems;
    type Error = ParseTomlError;

    fn parse(lines: Vec<String>) -> Result<Self::Item, Self::Error> {
        let toml_items = TomlItems::new(lines);
        Ok(toml_items)
    }
}

impl PartialEq for TomlItems {
    fn eq(&self, other: &TomlItems) -> bool {
        self.items == other.items
    }
}

#[derive(Debug, Clone)]
pub struct TomlHeader {
    pub extended: bool,
    pub inner: String,
    pub seg: Vec<String>,
}

impl<'p> Parse<String> for TomlHeader {

    type Item = TomlHeader;
    type Error = ParseTomlError;

    fn parse(header: String) -> Result<Self::Item, Self::Error> {
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

impl PartialEq for TomlHeader {
    fn eq(&self, other: &TomlHeader) -> bool {
        self.inner == other.inner
    }
}

#[derive(Debug, Clone,)]
pub struct TomlTable {
    pub header: TomlHeader,
    pub items: TomlItems,
}

impl TomlTable {
    pub fn sort_items(&mut self) {
        self.items.items.sort_unstable()
    }
}

impl PartialEq for TomlTable {
    fn eq(&self, other: &TomlTable) -> bool {
        self.header == other.header && self.items == other.items
    }
}

impl Eq for TomlTable {}

impl PartialOrd for TomlTable  {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TomlTable {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.header.inner.cmp(&other.header.inner)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_items_parse() {
        let item = r#"hello="{ why=yes }""#;

        let parsed = KVPair::parse(item);
        println!("{:#?}", parsed);
        
        //assert_ne!(unsorted, tt.tables)
    }
}