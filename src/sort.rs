use std::collections::BTreeMap;

use toml_edit::{Array, Document, Item, Table, Value};

use crate::Matcher;

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

fn gather_headings(table: &mut Table, keys: &mut Vec<Heading>) {
    for (head, item) in table.iter_mut() {
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

fn sort_arr(arr: &mut Array) {
    let mut sortable = Vec::with_capacity(arr.len());
    for _ in 0..arr.iter().count() {
        println!("{:?}", sortable);
        sortable.push(arr.remove(0));
    }
    sortable.sort_unstable_by(|a, b| a.as_str().cmp(&b.as_str()));
    for item in sortable {
        // Can't really fail here since we are using a previously valid `Array`
        arr.push_formatted(item).unwrap();
    }
}

/// Returns a sorted toml `Document`.
pub fn sort_toml(input: &str, matcher: Matcher) -> Document {
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
                        sort_arr(arr)
                    }
                }
            }
        }
    }

    let mut first_table = None;
    let mut heading_order: BTreeMap<_, Vec<Heading>> = BTreeMap::new();
    for (idx, (head, item)) in toml.as_table_mut().iter_mut().enumerate() {
        if !matcher.heading.contains(&head) {
            continue;
        }
        match item {
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
                table.sort_values();
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

    toml
}

#[cfg(test)]
mod test {
    use std::fs::{self};

    use super::Matcher;

    const HEADERS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

    const MATCHER: Matcher<'_> = Matcher {
        heading: &HEADERS,
        heading_key: &[("workspace", "members"), ("workspace", "exclude")],
    };

    #[test]
    fn toml_edit_check() {
        let input = fs::read_to_string("examp/work.toml").unwrap();
        let sorted = super::sort_toml(&input, MATCHER);
        assert_ne!(input, sorted.to_string_in_original_order());
        println!("{}", sorted.to_string_in_original_order());
    }
}
