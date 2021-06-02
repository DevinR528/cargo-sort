use afl::fuzz;

mod fmt;
mod sort;
mod toml_edit;

use fmt::Config;
use toml_edit::Document;

fn main() {
    fuzz!(|data: &[u8]| {
        if let Ok(s) = std::str::from_utf8(data) {
            let s = s.replace("\r", "");
            if s.parse::<Document>().is_ok() {
                let mut toml = sort::sort_toml(&s, sort::MATCHER, false, &[]);
                fmt::fmt_toml(&mut toml, &Config::new());
                let s = toml.to_string_in_original_order();
                assert!(s.parse::<Document>().is_ok())
            }
        }
    });
}
