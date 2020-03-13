# Cargo Sort Check

[![Build Status](https://travis-ci.com/DevinR528/cargo-sort-ck.svg?branch=master)](https://travis-ci.com/DevinR528/cargo-sort-ck)
[![Latest Version](https://img.shields.io/crates/v/cargo-sort-ck.svg)](https://crates.io/crates/cargo-sort-ck)

A tool to check that your Cargo.toml dependencies are sorted alphabetically. Project created as a solution to @dtolnay's [request for implementation #29](https://github.com/dtolnay/request-for-implementation/issues/29).  Cross platform implementation, windows compatible.  Terminal coloring works on both cmd.exe and powershell.  Checks/sorts by key in tables and also nested table headers (does not sort the items in a nested header, sorts the table itself). `cargo sort-ck` uses [toml-parse](https://github.com/DevinR528/toml-parse) to turn the toml file into a [rowan](https://github.com/rust-analyzer/rowan) syntax tree, it then sorts tokens keeping whitespace and comments intact. If the `print` or `write` options are used the tree is _lightly_ formatted, fixing only formatting issues the sorting causes.


## Use
There are three modes cargo-sort-ck can be used in:
 * **default**
    - no flags set cargo-sort-ck will pass (exit 0) if .toml is sorted or fail if not (exit 1).
 * **-p or --print**
    - will print the *__sorted toml__* file to stdout.
 * **-w or --write**
    - will rewrite the toml file, I would like to eventually add some kind of check like cargo fix to warn if file is uncommitted/unsaved?.

[toml]: https://github.com/toml-lang/toml
included in sort check is:
```toml
["dependencies"]
["dev-dependencies"]
["build-dependencies"]
["workspace.members"]
["workspace.exclude"]
```
if you have a header to add open a PR's, they are welcomed.


# Install
```bash
cargo install cargo-sort-ck
```

# Run
Defaults to current dir but any path can be passed in.
```bash
Cargo Sort Check 
Devin R <devin.ragotzy@gmail.com>
Ensure Cargo.toml dependency tables are sorted.

USAGE:
    cargo-sort-ck [FLAGS] [CWD]

FLAGS:
        --crlf       output uses windows style line endings (\\r\\n)
    -h, --help       Prints help information
    -p, --print      prints Cargo.toml, lexically sorted, to the screen
    -V, --version    Prints version information
    -w, --write      rewrites Cargo.toml file so it is lexically sorted

ARGS:
    <CWD>...    Sets cwd, must contain Cargo.toml
```
Thanks to [dspicher](https://github.com/dspicher) for [issue #4](https://github.com/DevinR528/cargo-sort-ck/issues/4) you can now invoke cargo sort check as a cargo subcommand
```bash
cargo sort-ck [FLAGS] [path]
```
Wildcard expansion is supported so you can do this
```bash
cargo-sort-ck [FLAGS] [path/to/*/Cargo.toml | path/to/*]
```
or any other pattern that is supported by your terminal. This also means multiple
paths work.
```bash
cargo-sort-ck [FLAGS] path/to/a path/to/b path/to/c/Cargo.toml
```
These are all valid, file name and extension can be used on some of the paths but not others, if
left off the defaults to Cargo.toml.

# Examples
```toml
[dependencies]
a="0.1.1"
# comments will stay with the item
c="0.1.1"

# if this key value is moved the whitespace before and after will stick
# unless it is at the end of a table then it is formatted.
b="0.1.1"

[dependencies.alpha]
version="0"

[build-dependencies]
foo="0"
bar="0"

# comments will also stay with header
[dependencies.zed]
version="0"

[dependencies.beta]
version="0"

[dev-dependencies]
bar="0"
foo="0"

```
Will sort to, or fail until organized like so
```toml
[dependencies]
a="0.1.1"

# if this key value is moved the whitespace before and after will stick
# unless it is at the end of a table then it is formatted.
b="0.1.1"
# comments will stay with the item
c="0.1.1"

[dependencies.alpha]
version="0"

[dependencies.beta]
version="0"

# comments will also stay with header
[dependencies.zed]
version="0"

[build-dependencies]
bar="0"
foo="0"

[dev-dependencies]
bar="0"
foo="0"

```

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
</sub>
