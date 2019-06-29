use std::error::Error;
use std::io;

pub enum TomlErrorKind {
    UnexpectedToken(String),
    InternalParseError(String),
}

pub struct ParseTomlError {
    kind: TomlErrorKind,
    info: String,
}

impl ParseTomlError {
    pub fn new(s: String, t_err: TomlErrorKind) -> ParseTomlError {
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
            TomlErrorKind::InternalParseError("? op Error".to_owned()),
        )
    }
}

impl std::convert::From<ParseTomlError> for io::Error {
    fn from(_e: ParseTomlError) -> io::Error {
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
