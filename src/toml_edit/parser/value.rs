use combine::{range::recognize_with_value, stream::RangeStream, *};

use crate::toml_edit::{
    decor::{Formatted, Repr},
    formatted,
    parser::{
        array::array,
        datetime::date_time,
        inline_table::inline_table,
        numbers::{boolean, float, integer},
        strings::string,
    },
    value as v,
};

// val = string / boolean / array / inline-table / date-time / float / integer
parse!(value() -> v::Value, {
    recognize_with_value(choice((
        string()
            .map(|s|
                 v::Value::String(Formatted::new(
                     s,
                     Repr::new("".to_string(), "who cares?".into(), "".to_string()),
                 ))
            ),
        boolean()
            .map(v::Value::from),
        array()
            .map(v::Value::Array),
        inline_table()
            .map(v::Value::InlineTable),
        date_time()
            .map(v::Value::from),
        float()
            .map(v::Value::from),
        integer()
            .map(v::Value::from),
    ))).map(|(raw, value)| formatted::value(value, raw))
});
