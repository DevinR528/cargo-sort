//! Runs the `cargo-sort` binary with `--grouped` over manifests that use
//! `[target.'cfg(..)'.dependencies]` tables (issue #137).

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const SORTED_GROUPS: &str = "\
[package]
name = \"sorttest\"
version = \"0.1.0\"
edition = \"2021\"

[target.'cfg(target_family = \"wasm\")'.dependencies]
web-wt-sys = \"0.3\"
xwt-core = \"0.9\"

js-sys = \"0.3\"
thiserror = \"2\"
";

const UNSORTED_GROUP: &str = "\
[package]
name = \"sorttest\"
version = \"0.1.0\"
edition = \"2021\"

[target.'cfg(target_family = \"wasm\")'.dependencies]
web-wt-sys = \"0.3\"
xwt-core = \"0.9\"

thiserror = \"2\"
js-sys = \"0.3\"
";

/// A directory holding one `Cargo.toml`, removed when dropped.
struct Project(PathBuf);

impl Project {
    fn new(name: &str, manifest: &str) -> Self {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("grouped-{name}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("Cargo.toml"), manifest).unwrap();
        Project(root)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cargo-sort"))
            .arg("sort")
            .args(args)
            .arg(&self.0)
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    }

    fn manifest(&self) -> String {
        fs::read_to_string(self.0.join("Cargo.toml")).unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}

#[test]
fn check_accepts_sorted_groups_in_target_tables() {
    let project = Project::new("check-ok", SORTED_GROUPS);
    let out = project.run(&["--check", "--grouped"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn check_rejects_an_unsorted_group_in_a_target_table() {
    let project = Project::new("check-err", UNSORTED_GROUP);
    let out = project.run(&["--check", "--grouped"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("not sorted"));
}

#[test]
fn write_keeps_groups_apart_in_target_tables() {
    let project = Project::new("write", UNSORTED_GROUP);
    let out = project.run(&["--grouped"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(project.manifest(), SORTED_GROUPS);
}

#[test]
fn without_grouped_the_target_table_is_checked_as_a_whole() {
    let project = Project::new("ungrouped", SORTED_GROUPS);
    let out = project.run(&["--check"]);
    assert!(!out.status.success(), "the groups are only sorted individually");
}
