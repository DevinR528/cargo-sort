use crate::toml_edit::{Key, Value};

macro_rules! parse {
    ($s:expr, $ty:ty) => {{
        let v = $s.parse::<$ty>();
        assert!(v.is_ok());
        v.unwrap()
    }};
}

macro_rules! test_key {
    ($s:expr, $expected:expr) => {{
        let key = parse!($s, Key);
        assert_eq!(key.get(), $expected);
    }};
}

macro_rules! parse_error {
    ($input:expr, $ty:ty, $err_msg:expr) => {{
        let res = $input.parse::<$ty>();
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.to_string().find($err_msg).is_some());
    }};
}

#[test]
fn test_parse_error() {
    parse_error!(r#"["", 2]"#, Value, "Mixed types in array");
    parse_error!("'hello'bla", Value, "Could not parse the line");
    parse_error!(r#"{a = 2"#, Value, "Expected `}`");

    parse_error!("'\"", Key, "Could not parse the line");
}

#[test]
fn test_key_from_str() {
    test_key!("a", "a");
    test_key!(r#"'hello key'"#, "hello key");
    test_key!(
        r#""Jos\u00E9\U000A0000\n\t\r\f\b\\\/\"""#,
        "Jos\u{00E9}\u{A0000}\n\t\r\u{c}\u{8}\\/\""
    );
    test_key!("", "");
    test_key!("'hello key'bla", "'hello key'bla");
    let wp = "C:\\Users\\appveyor\\AppData\\Local\\Temp\\1\\cargo-edit-test.YizxPxxElXn9";
    test_key!(wp, wp);
}
