use afl::fuzz;

mod fmt;
mod sort;

use fmt::Config;
use toml_edit::Document;

// cargo afl build --bin=fuzz --features=fuzz
// cargo afl fuzz -i examp/ -o target/cargo-sort-fuzz -- target/debug/fuzz
fn main() {
    fuzz!(|data: &[u8]| {
        if let Ok(s) = std::str::from_utf8(data) {
            let s = s.replace("\r", "");
            if s.parse::<Document>().is_ok() {
                let mut toml = sort::sort_toml(
                    &s,
                    sort::MATCHER,
                    false,
                    &[
                        "package".to_owned(),
                        "features".to_owned(),
                        "dependencies".to_owned(),
                        "build-dependencies".to_owned(),
                        "dev-dependencies".to_owned(),
                    ],
                );
                fmt::fmt_toml(&mut toml, &Config::new());
                let s = toml.to_string();
                assert!(s.parse::<Document>().is_ok())
            }
        }
    });
}
