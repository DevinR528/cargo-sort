use std::str::FromStr;

use combine::stream::state::State;

use super::{decor::InternalString, parser};

/// Key as part of a Key/Value Pair or a table header.
///
/// # Examples
///
/// ```notrust
/// [dependencies."nom"]
/// version = "5.0"
/// 'literal key' = "nonsense"
/// "basic string key" = 42
/// ```
///
/// There are 3 types of keys:
///
/// 1. Bare keys (`version` and `dependencies`)
///
/// 2. Basic quoted keys (`"basic string key"` and `"nom"`)
///
/// 3. Literal quoted keys (`'literal key'`)
///
/// For details see [toml spec](https://github.com/toml-lang/toml/#keyvalue-pair).
///
/// To parse a key use `FromStr` trait implementation: `"string".parse::<Key>()`.
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Clone)]
pub struct Key {
    key: InternalString,
    raw: InternalString,
}

impl FromStr for Key {
    type Err = parser::TomlError;

    /// Tries to parse a key from a &str,
    /// if fails, tries as basic quoted key (surrounds with "")
    /// and then literal quoted key (surrounds with '')
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Key::try_parse(s)
            .or_else(|_| Key::try_parse(&format!("\"{}\"", s)))
            .or_else(|_| Key::try_parse(&format!("'{}'", s)))
    }
}

impl Key {
    fn try_parse(s: &str) -> Result<Key, parser::TomlError> {
        use combine::Parser;
        let result = parser::key_parser().easy_parse(State::new(s));
        match result {
            Ok((_, ref rest)) if !rest.input.is_empty() => {
                Err(parser::TomlError::from_unparsed(rest.positioner, s))
            }
            Ok(((raw, key), _)) => Ok(Key::new(raw, key)),
            Err(e) => Err(parser::TomlError::new(e, s)),
        }
    }

    pub(crate) fn new(raw: &str, key: InternalString) -> Self {
        Self { raw: raw.into(), key }
    }

    /// Returns the parsed key value.
    pub fn get(&self) -> &str { &self.key }

    /// Returns the key raw representation.
    pub fn raw(&self) -> &str { &self.raw }
}

#[doc(hidden)]
impl From<Key> for InternalString {
    fn from(key: Key) -> InternalString { key.key }
}
