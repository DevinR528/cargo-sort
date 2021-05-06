use std::str::FromStr;

use chrono::{self, FixedOffset};
use combine::stream::state::State;
use linked_hash_map::LinkedHashMap;

use super::{
    decor::{Decor, Formatted, InternalString},
    formatted, parser,
    table::{Item, KeyValuePairs, TableKeyValue},
};

/// Representation of a TOML Value (as part of a Key/Value Pair).
#[derive(Debug, Clone)]
pub enum Value {
    /// A 64-bit integer value.
    Integer(Formatted<i64>),
    /// A string value.
    String(Formatted<String>),
    /// A 64-bit float value.
    Float(Formatted<f64>),
    /// A Date-Time value.
    DateTime(Formatted<DateTime>),
    /// A boolean value.
    Boolean(Formatted<bool>),
    /// An inline array of values.
    Array(Array),
    /// An inline table of key/value pairs.
    InlineTable(InlineTable),
}

/// Type representing a TOML Date-Time,
/// payload of the `Value::DateTime` variant's value
#[derive(Eq, PartialEq, Clone, Debug, Hash)]
pub enum DateTime {
    /// An RFC 3339 formatted date-time with offset.
    OffsetDateTime(chrono::DateTime<FixedOffset>),
    /// An RFC 3339 formatted date-time without offset.
    LocalDateTime(chrono::NaiveDateTime),
    /// Date portion of an RFC 3339 formatted date-time.
    LocalDate(chrono::NaiveDate),
    /// Time portion of an RFC 3339 formatted date-time.
    LocalTime(chrono::NaiveTime),
}

/// Type representing a TOML array,
/// payload of the `Value::Array` variant's value
#[derive(Debug, Default, Clone)]
pub struct Array {
    /// always Vec<Item::Value>
    pub values: Vec<Item>,
    /// `trailing` represents whitespaces, newlines
    /// and comments in an empty array or after the trailing comma
    pub trailing: InternalString,
    pub trailing_comma: bool,
    // prefix before `[` and suffix after `]`
    pub decor: Decor,
}

/// Type representing a TOML inline table,
/// payload of the `Value::InlineTable` variant
#[derive(Debug, Default, Clone)]
pub struct InlineTable {
    pub(crate) items: KeyValuePairs,
    // `preamble` represents whitespaces in an empty table
    pub(crate) preamble: InternalString,
    // prefix before `{` and suffix after `}`
    pub(crate) decor: Decor,
}

#[derive(Eq, PartialEq, Clone, Copy, Debug, Hash)]
pub(crate) enum ValueType {
    None,
    Integer,
    String,
    Float,
    DateTime,
    Boolean,
    Array,
    InlineTable,
}

impl Array {
    /// Returns the length of the underlying Vec.
    /// To get the actual number of items use `a.iter().count()`.
    pub fn len(&self) -> usize { self.values.len() }

    /// Return true iff `self.len() == 0`.
    pub fn is_empty(&self) -> bool { self.len() == 0 }

    /// Returns an iterator over all values.
    pub fn iter(&self) -> impl Iterator<Item = &Value> {
        self.values.iter().filter_map(Item::as_value)
    }

    /// Sorts a `Value::String` array.
    ///
    /// It uses the `Value::as_str` method to compare and sort.
    pub fn sort(&mut self) { self.values.sort_by(|a, b| a.as_str().cmp(&b.as_str())) }

    /// Appends a new, already formatted value to the end of the array.
    ///
    /// Returns an error if the value was of a different type than the array.
    pub fn push_formatted(&mut self, v: Value) -> Result<(), Value> {
        self.value_op(v, false, |items, value| items.push(Item::Value(value)))
    }

    /// Auto formats the array.
    pub fn fmt(&mut self, is_compact: bool) {
        formatted::decorate_array(self, is_compact);
    }

    fn value_op<T>(
        &mut self,
        v: Value,
        decorate: bool,
        op: impl FnOnce(&mut Vec<Item>, Value) -> T,
    ) -> Result<T, Value> {
        let mut value = v;
        if !self.is_empty() && decorate {
            formatted::decorate(&mut value, " ", "");
        } else if decorate {
            formatted::decorate(&mut value, "", "");
        }
        if self.is_empty() || value.get_type() == self.value_type() {
            Ok(op(&mut self.values, value))
        } else {
            Err(value)
        }
    }

    pub(crate) fn value_type(&self) -> ValueType {
        if let Some(value) = self.values.get(0).and_then(Item::as_value) {
            value.get_type()
        } else {
            ValueType::None
        }
    }
}

impl InlineTable {
    /// Returns the number of key/value pairs.
    pub fn len(&self) -> usize { self.iter().count() }

    /// Returns an iterator over key/value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.items
            .iter()
            .filter(|&(_, kv)| kv.value.is_value())
            .map(|(k, kv)| (&k[..], kv.value.as_value().unwrap()))
    }

    /// Returns true iff the table contains given key.
    pub fn contains_key(&self, key: &str) -> bool {
        if let Some(kv) = self.items.get(key) { !kv.value.is_none() } else { false }
    }

    /// Auto formats the table.
    pub fn fmt(&mut self, is_compact: bool) {
        formatted::decorate_inline_table(self, is_compact);
    }
}

impl Value {
    /// Casts `self` to integer.
    pub fn as_integer(&self) -> Option<i64> {
        match *self {
            Value::Integer(ref value) => Some(*value.value()),
            _ => None,
        }
    }

    /// Casts `self` to boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Value::Boolean(ref value) => Some(*value.value()),
            _ => None,
        }
    }

    /// Casts `self` to str.
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Value::String(ref value) => Some(value.value()),
            _ => None,
        }
    }

    /// Casts `self` to array.
    pub fn as_array(&self) -> Option<&Array> {
        match *self {
            Value::Array(ref value) => Some(value),
            _ => None,
        }
    }

    /// Casts `self` to mutable array.
    pub fn as_array_mut(&mut self) -> Option<&mut Array> {
        match *self {
            Value::Array(ref mut value) => Some(value),
            _ => None,
        }
    }

    /// Returns true iff `self` is an array.
    pub fn is_array(&self) -> bool { self.as_array().is_some() }

    /// Casts `self` to inline table.
    pub fn as_inline_table(&self) -> Option<&InlineTable> {
        match *self {
            Value::InlineTable(ref value) => Some(value),
            _ => None,
        }
    }

    /// Casts `self` to mutable inline table.
    pub fn as_inline_table_mut(&mut self) -> Option<&mut InlineTable> {
        match *self {
            Value::InlineTable(ref mut value) => Some(value),
            _ => None,
        }
    }

    /// Returns true iff `self` is an inline table.
    pub fn is_inline_table(&self) -> bool { self.as_inline_table().is_some() }

    pub(crate) fn get_type(&self) -> ValueType {
        match *self {
            Value::Integer(..) => ValueType::Integer,
            Value::String(..) => ValueType::String,
            Value::Float(..) => ValueType::Float,
            Value::DateTime(..) => ValueType::DateTime,
            Value::Boolean(..) => ValueType::Boolean,
            Value::Array(..) => ValueType::Array,
            Value::InlineTable(..) => ValueType::InlineTable,
        }
    }
}

impl Value {
    /// Get the decoration of the value.
    /// # Example
    /// ```rust
    /// let v = toml_edit::Value::from(true);
    /// assert_eq!(v.decor().suffix(), "");
    /// ```
    pub fn decor(&self) -> &Decor {
        match self {
            Value::Integer(f) => &f.repr.decor,
            Value::String(f) => &f.repr.decor,
            Value::Float(f) => &f.repr.decor,
            Value::DateTime(f) => &f.repr.decor,
            Value::Boolean(f) => &f.repr.decor,
            Value::Array(a) => &a.decor,
            Value::InlineTable(t) => &t.decor,
        }
    }

    pub fn decor_mut(&mut self) -> &mut Decor {
        match self {
            Value::Integer(f) => &mut f.repr.decor,
            Value::String(f) => &mut f.repr.decor,
            Value::Float(f) => &mut f.repr.decor,
            Value::DateTime(f) => &mut f.repr.decor,
            Value::Boolean(f) => &mut f.repr.decor,
            Value::Array(a) => &mut a.decor,
            Value::InlineTable(t) => &mut t.decor,
        }
    }
}

pub(crate) fn sort_key_value_pairs(
    items: &mut LinkedHashMap<InternalString, TableKeyValue>,
) {
    let mut keys: Vec<_> = items
        .iter()
        .filter_map(|i| (i.1).value.as_value().map(|_| i.0))
        .cloned()
        .collect();
    keys.sort();
    for key in keys {
        items.get_refresh(&key);
    }
}

impl FromStr for Value {
    type Err = parser::TomlError;

    /// Parses a value from a &str
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use combine::Parser;
        let parsed = crate::toml_edit::parser::value_parser().easy_parse(State::new(s));
        match parsed {
            Ok((_, ref rest)) if !rest.input.is_empty() => {
                Err(Self::Err::from_unparsed(rest.positioner, s))
            }
            Ok((value, _)) => Ok(value),
            Err(e) => Err(Self::Err::new(e, s)),
        }
    }
}
