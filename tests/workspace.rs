//! Runs the `cargo-sort` binary with `--workspace` over manifests written to disk.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const SORTED: &str = "[dependencies]\nanyhow = \"1\"\nserde = \"1\"\n";
const UNSORTED: &str = "[dependencies]\nserde = \"1\"\nanyhow = \"1\"\n";

/// A directory of `Cargo.toml` files for one test, removed when dropped.
struct Tree(PathBuf);

impl Tree {
    /// `manifests` are (directory, `Cargo.toml` contents) pairs.
    fn new(name: &str, manifests: &[(&str, &str)]) -> Self {
        let root =
            Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("workspace-{name}"));
        let _ = fs::remove_dir_all(&root);
        for (dir, manifest) in manifests {
            let dir = root.join(dir);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("Cargo.toml"), manifest).unwrap();
        }
        Tree(root)
    }

    /// Runs `cargo sort <args>` in `dir` (relative to the tree).
    fn run(&self, dir: &str, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cargo-sort"))
            .arg("sort")
            .args(args)
            .current_dir(self.0.join(dir))
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    }

    fn read(&self, dir: &str) -> String {
        fs::read_to_string(self.0.join(dir).join("Cargo.toml")).unwrap()
    }
}

impl Drop for Tree {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}

/// The crate names the binary reported as checked, in order.
fn checked(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|l| l.strip_prefix("Checking ")?.strip_suffix("..."))
        .map(str::to_owned)
        .collect()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Issue #139: a workspace with an unsorted member, plus an excluded crate that is its
/// own workspace.
fn issue_139_tree(name: &str) -> Tree {
    Tree::new(
        name,
        &[
            ("ws", "[workspace]\nmembers = [\"a\", \"b\"]\nexclude = [\"extra\"]\n"),
            ("ws/a", SORTED),
            ("ws/b", UNSORTED),
            ("ws/extra", &format!("[workspace]\n\n{SORTED}")),
        ],
    )
}

#[test]
fn unlisted_member_is_checked_with_several_paths() {
    let tree = issue_139_tree("issue-139");
    let out = tree.run("ws", &["-w", "--check", ".", "extra"]);
    assert_eq!(checked(&out), [".", "extra", "a", "b"]);
    assert!(
        stderr(&out).contains("Dependencies for b are not sorted"),
        "{}",
        stderr(&out)
    );
    assert!(!out.status.success());
}

#[test]
fn members_of_every_workspace_are_checked() {
    let tree = Tree::new(
        "two-workspaces",
        &[
            ("ws", "[workspace]\nmembers = [\"a\"]\n"),
            ("ws/a", SORTED),
            ("ws2", "[workspace]\nmembers = [\"crates/*\"]\n"),
            ("ws2/crates/c", SORTED),
            ("ws2/crates/d", UNSORTED),
        ],
    );
    let out = tree.run(".", &["-w", "--check", "ws", "ws2"]);
    assert_eq!(checked(&out), ["ws", "ws2", "a", "c", "d"]);
    assert!(
        stderr(&out).contains("Dependencies for d are not sorted"),
        "{}",
        stderr(&out)
    );
    assert!(!out.status.success());

    // Without --check, the unsorted member of the second workspace is rewritten
    let out = tree.run(".", &["-w", "ws", "ws2"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(tree.read("ws2/crates/d"), SORTED);
    let out = tree.run(".", &["-w", "--check", "ws", "ws2"]);
    assert!(out.status.success(), "{}", stderr(&out));
}

#[test]
fn crate_without_workspace_is_checked_alongside_a_workspace() {
    let tree = Tree::new(
        "plain-crate",
        &[
            ("ws", "[workspace]\nmembers = [\"a\"]\n"),
            ("ws/a", SORTED),
            ("plain", UNSORTED),
        ],
    );
    let out = tree.run(".", &["-w", "--check", "plain", "ws"]);
    assert_eq!(checked(&out), ["plain", "ws", "a"]);
    assert!(!out.status.success());
}

#[test]
fn manifest_reached_twice_is_checked_once() {
    let tree = issue_139_tree("reached-twice");
    let out = tree.run("ws", &["-w", "--check", ".", "b", "./"]);
    assert_eq!(checked(&out), [".", "b", "a"]);
    assert_eq!(stderr(&out).matches("Dependencies for b are not sorted").count(), 1);
    assert!(!out.status.success());
}

#[test]
fn workspace_flag_needs_a_workspace() {
    let tree = Tree::new("no-workspace", &[("plain", SORTED), ("plain2", SORTED)]);
    for args in [&["-w", "--check", "plain", "plain2"][..], &["-w", "--check", "plain"]] {
        let out = tree.run(".", args);
        assert!(checked(&out).is_empty(), "{args:?}: {:?}", checked(&out));
        assert!(stderr(&out).contains("--workspace was given"), "{}", stderr(&out));
        assert!(!out.status.success());
    }
    let out = tree.run("plain", &["-w", "--check"]);
    assert!(stderr(&out).contains("--workspace was given"), "{}", stderr(&out));
    assert!(!out.status.success());
}

#[test]
fn paths_without_workspace_flag_are_checked_as_given() {
    let tree = issue_139_tree("no-flag");
    let out = tree.run("ws", &["--check", ".", "extra"]);
    assert_eq!(checked(&out), [".", "extra"]);
    assert!(out.status.success(), "{}", stderr(&out));
}
