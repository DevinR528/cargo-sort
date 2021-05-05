use std::collections::BTreeMap;

use fmt::Config;
use toml_edit::{Array, Document, Item, Table, Value};

use crate::{fmt, Matcher};

/// A state machine to track collection of headings.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Heading {
    /// After collecting heading segments we recurse into another table.
    Next(Vec<String>),
    /// We have found a completed heading.
    ///
    /// The the heading we are processing has key value pairs.
    Complete(Vec<String>),
}

fn gather_headings(table: &Table, keys: &mut Vec<Heading>) {
    for (head, item) in table.iter() {
        match item {
            Item::Value(_) => {
                if keys.last().map_or(false, |h| matches!(h, Heading::Complete(_))) {
                    continue;
                }
                let next = match keys.pop().unwrap() {
                    Heading::Next(segs) => Heading::Complete(segs),
                    _complete => unreachable!("the above if check prevents this"),
                };
                keys.push(next);
                continue;
            }
            Item::Table(table) => {
                let next = match keys.pop().unwrap() {
                    Heading::Next(mut segs) => {
                        segs.push(head.into());
                        Heading::Next(segs)
                    }
                    // This happens when
                    //
                    // [heading]       // transitioning from here to
                    // [heading.segs]  // here
                    Heading::Complete(segs) => {
                        let next = vec![segs[0].clone(), head.into()];
                        keys.push(Heading::Complete(segs));
                        Heading::Next(next)
                    }
                };
                keys.push(next);
                gather_headings(table, keys);
            }
            Item::ArrayOfTables(arr) => todo!("ArrayOfTables: {:?}", arr),
            Item::None => panic!("{:?}", keys),
        }
    }
}

fn sort_arr(arr: &mut Array) {}

fn sort_by_group(table: &mut Table) {
    let table_clone = table.clone();
    let mut groups = BTreeMap::new();
    let mut curr = 0;
    for (idx, (k, v)) in
        table_clone.iter().map(|(k, _)| (k, table.remove_full(k).unwrap())).enumerate()
    {
        let decor = v.decor();
        if decor.prefix().contains('\n') {
            groups.entry(idx).or_insert_with(|| vec![(k, v)]);
            curr = idx;
        } else {
            groups.entry(curr).or_default().push((k, v));
        }
    }

    let mut first_kv_pair = true;
    for (_, mut group) in groups {
        group.sort_by(|a, b| a.0.cmp(&b.0));
        for (i, (k, mut v)) in group.into_iter().enumerate() {
            let dec = v.decor_mut();

            if i == 0 && !first_kv_pair {
                dec.prefix = "\n".to_string();
            } else {
                dec.prefix = String::new();
            }
            table.insert_key_value(k, v);
        }
        first_kv_pair = false;
    }
}

/// Returns a sorted toml `Document`.
pub fn sort_toml(
    input: &str,
    matcher: Matcher,
    config: &Config,
    group: bool,
    format: bool,
) -> Document {
    let mut toml = input.parse::<Document>().unwrap();
    // This takes care of `[workspace] members = [...]`
    for (heading, key) in matcher.heading_key {
        // Since this `&mut toml[&heading]` is like
        // `SomeMap.entry(key).or_insert(Item::None)` we only want to do it if we
        // know the heading is there already
        if toml.as_table().contains_key(heading) {
            if let Item::Table(table) = &mut toml[heading] {
                if table.contains_key(key) {
                    if let Item::Value(Value::Array(arr)) = &mut table[key] {
                        arr.sort();
                    }
                }
            }
        }
    }

    let mut first_table = None;
    let mut heading_order: BTreeMap<_, Vec<Heading>> = BTreeMap::new();
    for (idx, (head, item)) in toml.as_table_mut().iter_mut().enumerate() {
        if !matcher.heading.contains(&head.as_str()) {
            continue;
        }
        match item.value_mut() {
            Item::Table(table) => {
                if first_table.is_none() {
                    first_table = Some(idx);
                }
                let headings = heading_order.entry((idx, head.to_string())).or_default();
                headings.push(Heading::Complete(vec![head.to_string()]));
                // Push a `Heading::Complete` here incase the tables are ordered
                // [heading.segs]
                // [heading]

                gather_headings(table, headings);
                headings.sort();
                if group {
                    sort_by_group(table);
                } else {
                    // Sort also removes any newlines between key value pairs
                    table.sort_values();
                }
            }
            Item::None => continue,
            _ => unreachable!("Top level toml must be tables"),
        }
    }

    // Since the root table is always index 0 we add one
    let first_table_idx = first_table.unwrap_or_default() + 1;
    for (idx, heading) in heading_order.into_iter().flat_map(|(_, segs)| segs).enumerate()
    {
        // println!("{:?} {}", heading, first_table_idx);
        if let Heading::Complete(segs) = heading {
            let mut table = toml.as_table_mut();
            for seg in segs {
                // We know these are valid tables since we just collected them
                table = table[&seg].as_table_mut().unwrap();
            }
            table.set_position(first_table_idx + idx);
        }
    }

    if format {
        fmt::fmt_toml(&mut toml, config)
    }

    toml
}

#[cfg(test)]
mod test {
    use std::fs::{self};

    use super::{fmt::Config, Matcher};

    const HEADERS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

    const MATCHER: Matcher<'_> = Matcher {
        heading: &HEADERS,
        heading_key: &[("workspace", "members"), ("workspace", "exclude")],
    };

    const CONFIG: Config = Config {
        trailing_comma: false,
        space_around_eq: true,
        compact_arrays: false,
        compact_inline_tables: false,
        trailing_newline: true,
        key_value_newlines: true,
        allowed_blank_lines: 1,
        crlf: false,
    };

    #[test]
    fn toml_edit_check() {
        let input = fs::read_to_string("examp/workspace.toml").unwrap();
        let sorted = super::sort_toml(&input, MATCHER, &CONFIG, false, false);
        assert_ne!(input, sorted.to_string_in_original_order());
        println!("{}", sorted.to_string_in_original_order());
    }

    #[test]
    fn grouped_check() {
        let input = fs::read_to_string("examp/ruma.toml").unwrap();
        let sorted = super::sort_toml(&input, MATCHER, &CONFIG, true, false);
        println!("{}", sorted.to_string_in_original_order());
    }
}
