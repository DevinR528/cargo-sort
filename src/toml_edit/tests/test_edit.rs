macro_rules! as_table {
    ($e:expr) => {{
        assert!($e.is_table());
        $e.as_table_mut().unwrap()
    }};
}

// rusfmt, U Can't Touch This
#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use crate::toml_edit::{ArrayOfTables, Document, Item, Table, value};
    use std::fmt;
    use pretty_assertions::assert_eq;

    fn table() -> Item { Item::Table(Table::new()) }

    // Copied from https://github.com/colin-kiegel/rust-pretty-assertions/issues/24
    /// Wrapper around string slice that makes debug output `{:?}` to print string same way as `{}`.
    /// Used in different `assert*!` macros in combination with `pretty_assertions` crate to make
    /// test failures to show nice diffs.
    #[derive(PartialEq, Eq)]
    struct PrettyString<'a>(pub &'a str);
    /// Make diff to display string as multi-line string
    impl<'a> fmt::Debug for PrettyString<'a> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }

    struct Test {
        doc: Document,
    }

    fn given(input: &str) -> Test {
        let doc = input.parse::<Document>();
        assert!(doc.is_ok());
        Test {
            doc: doc.unwrap(),
        }
    }

    impl Test {
        fn running<F>(&mut self, func: F) -> &mut Self
            where F: Fn(&mut Table)
        {
            {
                let root = self.doc.as_table_mut();
                func(root);
            }
            self
        }

        fn produces_display(&self, expected: &str) -> &Self {
            assert_eq!(
                PrettyString(expected),
                PrettyString(&self.doc.to_string()));
            self
        }

        fn produces_in_original_order(&self, expected: &str) -> &Self {
            assert_eq!(
                PrettyString(expected),
                PrettyString(&self.doc.to_string_in_original_order()));
            self
        }

        fn produces(&self, expected: &str) -> &Self {
            self.produces_display(expected).produces_in_original_order(expected);
            self
        }
    }

// insertion

#[test]
fn test_insert_leaf_table() {
    given(r#"
        [servers]

        [servers.alpha]
        ip = "10.0.0.1"
        dc = "eqdc10"

        [other.table]"#
    ).running(|root| {
        root["servers"]["beta"] = table();
        root["servers"]["beta"]["ip"] = value("10.0.0.2");
        root["servers"]["beta"]["dc"] = value("eqdc10");
    }).produces(r#"
        [servers]

        [servers.alpha]
        ip = "10.0.0.1"
        dc = "eqdc10"

[servers.beta]
ip = "10.0.0.2"
dc = "eqdc10"

        [other.table]
"#
    );
}

#[test]
fn test_inserted_leaf_table_goes_after_last_sibling() {
    given(r#"
        [package]
        [dependencies]
        [[example]]
        [dependencies.opencl]
        [dev-dependencies]"#
    ).running(|root| {
        root["dependencies"]["newthing"] = table();
    }).produces_display(r#"
        [package]
        [dependencies]
        [dependencies.opencl]

[dependencies.newthing]
        [[example]]
        [dev-dependencies]
"#).produces_in_original_order(r#"
        [package]
        [dependencies]
        [[example]]
        [dependencies.opencl]

[dependencies.newthing]
        [dev-dependencies]
"#);
}

#[test]
fn test_inserting_tables_from_different_parsed_docs() {
    given(
        "[a]"
    ).running(|root| {
        let other = "[b]".parse::<Document>().unwrap();
        root["b"] = other["b"].clone();
    }).produces(
        "[a]\n[b]\n"
    );
}
#[test]
fn test_insert_nonleaf_table() {
    given(r#"
        [other.table]"#
    ).running(|root| {
        root["servers"] = table();
        root["servers"]["alpha"] = table();
        root["servers"]["alpha"]["ip"] = value("10.0.0.1");
        root["servers"]["alpha"]["dc"] = value("eqdc10");
    }).produces(r#"
        [other.table]

[servers]

[servers.alpha]
ip = "10.0.0.1"
dc = "eqdc10"
"#
    );
}

#[test]
fn test_insert_array() {
    given(r#"
        [package]
        title = "withoutarray""#
    ).running(|root| {
        root["bin"] = Item::ArrayOfTables(ArrayOfTables::new());
        let array = root["bin"].as_array_of_tables_mut().unwrap();
        {
            let first = array.append(Table::new());
            first["hello"] = value("world");
        }
        array.append(Table::new());
    }).produces(r#"
        [package]
        title = "withoutarray"

[[bin]]
hello = "world"

[[bin]]
"#
    );
}


#[test]
fn test_insert_values() {
    given(r#"
        [tbl.son]"#
    ).running(|root| {
        root["tbl"]["key1"] = value("value1");
        root["tbl"]["\"key2\""] = value(42);
        root["tbl"]["'key3'"] = value(8.1415926);
    }).produces(r#"
[tbl]
key1 = "value1"
"key2" = 42
'key3' = 8.1415926

        [tbl.son]
"#
    );
}

// values

#[test]
fn test_sort_values() {
    given(r#"
        [a.z]

        [a]
        # this comment is attached to b
        b = 2 # as well as this
        a = 1
        c = 3

        [a.y]"#
    ).running(|root| {
        let a = root.entry("a");
        let a = as_table!(a);
        a.sort_values();
    }).produces_display(r#"
        [a]
        a = 1
        # this comment is attached to b
        b = 2 # as well as this
        c = 3

        [a.z]

        [a.y]
"#
    ).produces_in_original_order(r#"
        [a.z]

        [a]
        a = 1
        # this comment is attached to b
        b = 2 # as well as this
        c = 3

        [a.y]
"#);
}

#[test]
fn test_set_position() {
    given(r#"
        [package]
        [dependencies]
        [dependencies.opencl]
        [dev-dependencies]"#
    ).running(|root| {
        for (header, table) in root.iter_mut() {
            if header == "dependencies" {
                let tab = as_table!(table.value_mut());
                tab.set_position(0);
                let (_, segmented) = tab.iter_mut().next().unwrap();
                as_table!(segmented.value_mut()).set_position(5)
            }
        }
    }).produces_in_original_order(r#"        [dependencies]

        [package]
        [dev-dependencies]
        [dependencies.opencl]
"#);
}

#[test]
fn test_multiple_zero_positions() {
    given(r#"
        [package]
        [dependencies]
        [dependencies.opencl]
        a=""
        [dev-dependencies]"#
    ).running(|root| {
        for (_, table) in root.iter_mut() {
            as_table!(table.value_mut()).set_position(0)
        }
    }).produces_in_original_order(r#"
        [package]
        [dependencies]
        [dev-dependencies]
        [dependencies.opencl]
        a=""
"#);
}

#[test]
fn test_multiple_max_usize_positions() {
    given(r#"
        [package]
        [dependencies]
        [dependencies.opencl]
        a=""
        [dev-dependencies]"#
    ).running(|root| {
        for (_, table) in root.iter_mut() {
            as_table!(table.value_mut()).set_position(usize::MAX)
        }
    }).produces_in_original_order(r#"        [dependencies.opencl]
        a=""

        [package]
        [dependencies]
        [dev-dependencies]
"#);
}

} // mod tests
