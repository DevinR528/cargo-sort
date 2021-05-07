use combine::{
    attempt, between, char, char::char, combine_parse_partial, combine_parser_impl,
    parse_mode, parser, sep_by, stream::RangeStream, ParseError, StreamOnce,
};

use crate::toml_edit::{
    decor::{InternalString, Repr},
    formatted::decorated,
    parser::{errors::CustomError, key::key, trivia::ws, value::value},
    table::{Item, TableKeyValue},
    value::InlineTable,
};

// ;; Inline Table

// inline-table = inline-table-open inline-table-keyvals inline-table-close
parse!(inline_table() -> InlineTable, {
    between(char(INLINE_TABLE_OPEN), char(INLINE_TABLE_CLOSE),
            inline_table_keyvals().and_then(|(p, v)| table_from_pairs(p, v)))
});

fn table_from_pairs(
    preamble: &str,
    v: Vec<(InternalString, TableKeyValue)>,
) -> Result<InlineTable, CustomError> {
    let mut table =
        InlineTable { preamble: InternalString::from(preamble), ..Default::default() };

    for (k, kv) in v {
        if table.contains_key(&k) {
            return Err(CustomError::DuplicateKey { key: k, table: "inline".into() });
        }
        table.items.insert(k, kv);
    }
    Ok(table)
}

// inline-table-open  = %x7B ws     ; {
const INLINE_TABLE_OPEN: char = '{';
// inline-table-close = ws %x7D     ; }
const INLINE_TABLE_CLOSE: char = '}';
// inline-table-sep   = ws %x2C ws  ; , Comma
const INLINE_TABLE_SEP: char = ',';
// keyval-sep = ws %x3D ws ; =
pub(crate) const KEYVAL_SEP: char = '=';

// inline-table-keyvals = [ inline-table-keyvals-non-empty ]
// inline-table-keyvals-non-empty =
// ( key keyval-sep val inline-table-sep inline-table-keyvals-non-empty ) /
// ( key keyval-sep val )

parse!(inline_table_keyvals() -> (&'a str, Vec<(InternalString, TableKeyValue)>), {
    (
        sep_by(keyval(), char(INLINE_TABLE_SEP)),
        ws(),
    ).map(|(v, w)| {
        (w, v)
    })
});

parse!(keyval() -> (InternalString, TableKeyValue), {
    (
        attempt((ws(), key(), ws())),
        char(KEYVAL_SEP),
        (ws(), value(), ws()),
    ).map(|(k, _, v)| {
        let (pre, v, suf) = v;
        let v = decorated(v, pre, suf);
        let (pre, (raw, key), suf) = k;
        (
            key,
            TableKeyValue {
                key: Repr::new(pre, raw, suf),
                value: Item::Value(v),
            }
        )
    })
});
