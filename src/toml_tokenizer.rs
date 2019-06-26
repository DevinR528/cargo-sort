use std::io;
use std::result::Result;
use std::error::Error;

#[cfg(windows)]
const EOL: &'static str = "\r\n";
#[cfg(not(windows))]
const EOL: &'static str = "\n";

#[derive(Debug, Clone)]
struct TomlItems<'s> {
    items: Vec<&'s str>,
}

impl<'s> TomlItems<'s> {
    fn new() -> TomlItems<'s> {
        TomlItems {
            items: Vec::default(),
        }
    }
}

#[derive(Debug, Clone)]
struct TomlHeader<'s> {
    inner: String,
    seg: Vec<&'s str>,
}


#[derive(Debug, Clone)]
struct TomlTable<'s> {
    header: TomlHeader<'s>,
    items: TomlItems<'s>,
}

impl<'s> TomlTable<'s> {

    fn add_items(&mut self, i: TomlItems<'s>) {
        self.items = i
    }

    fn sort_items(&mut self) {
        self.items.items.sort_unstable()
    }
}

#[derive(Debug)]
pub struct TomlTokenizer<'s> {
    tables: Vec<TomlTable<'s>>,
    loc: std::slice::IterMut<'s, &'s str>,
}

impl<'s> TomlTokenizer<'s> {


    pub fn get_items(&self) {

    }

    pub fn get_tables(&self) {

    }

    fn recurs_toml(
        &mut self,
        lines: Vec<&'s str>,
    ) -> Result<(/*TomlTokenizer*/), ParseTomlError> {
        while let Some(line) = lines.iter().next() {
            let letter = line.chars().next().unwrap();
            match letter {
                '[' => {
                    let mut header = TomlHeader::parse(&line)?;
                    println!("{:#?}", header);
                    let mut lines2 = lines.clone();
                    let items = TomlItems::parse(lines2)?;
                    println!("{:#?}", items);
                    let table = TomlTable {
                        header,
                        items: items.clone(),
                    };
                    self.tables.push(table);
                    println!("{:#?}", items);                  
                },
                '\r' | '\n' => {},//recurs_toml(lines, tt)?,
                _ => {},
            }
        }
        Ok(())
    }

    pub fn from_str(s: &str,) -> Result<(/*TomlTokenizer*/), ParseTomlError> {

        let lines: Vec<&str> = s.lines().collect();

        let mut lines_mut = lines.clone();
        let mut l_iter_mut = lines_mut.iter_mut();

        let mut tt = TomlTokenizer { tables: Vec::default(), loc: l_iter_mut };

        tt.recurs_toml(lines)?;

        Ok(())
    }

}

trait Parse<P=Self> {
    type Item;
    type Return;
    type Error;
    fn parse(s: Self::Item) -> Result<Self::Return, Self::Error>;
}

impl<'p> Parse for TomlHeader<'p> {

    type Item = &'p str;
    type Return = TomlHeader<'p>;
    type Error = ParseTomlError;

    fn parse(s: Self::Item) -> Result<Self::Return, Self::Error> {
        if s.contains(".") {
            let segmented = s.trim_matches(|c| c == '[' || c == ']');
            let seg = segmented.split(".").collect();
            println!("SEG {:#?}", seg);
            return Ok(TomlHeader { inner: s.to_string(), seg: seg })
        }
        let seg: Vec<&str> = s.trim_matches(|c| c == '[' || c == ']').split(".").collect();
        println!("SEG {:#?}", seg);
        if seg.is_empty() {
            let span = s.to_owned();
            return Err(
                ParseTomlError::new(
                    "No value inside header".to_owned(),
                    TomlErrorKind::UnexpectedToken(span))
            )
        }
        Ok(TomlHeader { inner: s.to_owned(), seg  })
    }
}

impl<'p> Parse for TomlItems<'p> {

    type Item = &'p mut std::slice::IterMut<'p, &'p str>;
    type Return = TomlItems<'p>;
    type Error = ParseTomlError;

    fn parse(lines: Self::Item) -> Result<Self::Return, Self::Error> {
        println!("IN ITEMS {:#?}", lines);
        let mut toml_items = TomlItems::new();

        loop {
            let item = lines.next().unwrap();
            if item.is_empty() || item.starts_with("[") {

                println!("END OF WHILE {:#?}", lines);
                return Ok(toml_items)
            } else {
                println!("ELSE {}", item);
                toml_items.items.push(item);
            }
        }
        // while let Some(item) = lines.next() {
        //     if item.starts_with("") || item.starts_with("[") {
        //         println!("END OF WHILE");
        //         return Ok(toml_items)
        //     } else {
        //         println!("ELSE {}", item);
        //         toml_items.items.push(item);
        //     }
        // } 
        Err(ParseTomlError::new(
                "No value inside header".to_owned(),
                TomlErrorKind::InternalParseError("".to_owned()))
            )
    }
}

enum TomlErrorKind {
    UnexpectedToken(String),
    InternalParseError(String),
}

pub struct ParseTomlError {
    kind: TomlErrorKind,
    info: String,
}

impl ParseTomlError {
    fn new(s: String, t_err: TomlErrorKind) -> ParseTomlError {
        ParseTomlError {
            kind: t_err,
            info: s,
        }
    }
}

impl std::convert::From<io::Error> for ParseTomlError {
    fn from(e: io::Error) -> ParseTomlError {
        let msg = e.description().to_owned();
        ParseTomlError::new(
            msg,
            TomlErrorKind::InternalParseError("? op Error".to_owned())
        )
    }
}

impl std::convert::From<ParseTomlError> for io::Error {
    fn from(e: ParseTomlError) -> io::Error {
        io::Error::new(io::ErrorKind::Other, "uh oh")
    }
}

impl std::error::Error for ParseTomlError {
    fn description(&self) -> &str {
        self.info.as_str()
    }
}

impl std::fmt::Debug for ParseTomlError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::fmt::Display for ParseTomlError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let span = match &self.kind {
            TomlErrorKind::InternalParseError(ref span) => span,
            TomlErrorKind::UnexpectedToken(ref span) => span,
        };
        write!(f, "{} caused by {}", self.info, span)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
   
    #[test]
    fn create_tokenizer() {

        let f = std::fs::read_to_string("examp/right.toml").expect("no file found");
        println!("{}", f);
        let tt = TomlTokenizer::from_str(&f);
    }
}