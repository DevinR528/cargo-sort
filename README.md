# Cargo Sort

[![Rust Stable](https://github.com/DevinR528/cargo-sort-ck/actions/workflows/stable.yml/badge.svg)](https://github.com/DevinR528/cargo-sort-ck/actions/workflows/stable.yml)

A tool to check that your Cargo.toml dependencies are sorted alphabetically. Project created as a solution to @dtolnay's [request for implementation #29](https://github.com/dtolnay/request-for-implementation/issues/29).  Cross platform implementation, windows compatible.  Terminal coloring works on both cmd.exe and powershell.  Checks/sorts by key in tables and also nested table headers (does not sort the items in a nested header, sorts the table itself). `cargo sort` uses [toml-edit](https://github.com/ordian/toml_edit) to parse the toml file into something useful.

The `--format` option may result in improperly formatted toml please file an issue.

## Use

There are three modes cargo-sort can be used in:
 * **default**
    - No flags set cargo-sort will write the sorted result over the input Cargo.toml file.
 * **-c or --check**
    - Will fail with a non-zero exit code if the file is unsorted.
 * **-n or --no-format**
    - Will **NOT** format the sorted toml. This option only has an effect if writing or printing out.
 * **-g or --grouped**
    - When sorting keep table key value spacing. If you have dependency groups they will stick but be sorted within the grouping.
* **-p or --print**
    - Write the sorted toml file to stdout.
* **-w or --workspace**
    - Checks every crate in the workspace based on flags. Only one root may be given.

### Config

cargo sort uses a config file when formatting called `tomlfmt.toml`. This is optional and defaults will
be used if not found in the current working dir.

Here are the defaults when no `tomlfmt.toml` is found
```toml
# trailing comma in arrays
alwyas_trailing_comma = false
# trailing comma when multi-line
multiline_trailing_comma = true
space_around_eq = true
# remove all the spacing inside the array
compact_arrays = false
# remove all the spacing inside the object
compact_inline_tables = false
trailing_newline = true
# is it ok to have blank lines inside of a table
key_value_newlines = false
allowed_blank_lines = 1
# windows style line endings
crlf = false
```

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
cargo install cargo-sort
```

# Run

Thanks to [dspicher](https://github.com/dspicher) for [issue #4](https://github.com/DevinR528/cargo-sort-ck/issues/4) you can now invoke cargo sort check as a cargo subcommand

```bash
cargo sort [FLAGS] [path]
```
Wildcard expansion is supported so you can do this
```bash
cargo-sort [FLAGS] [path/to/*/Cargo.toml | path/to/*]
```
or any other pattern that is supported by your terminal. This also means multiple
paths work.
```bash
cargo-sort [FLAGS] path/to/a path/to/b path/to/c/Cargo.toml
```
Finally cargo sort has the --workspace flag and will sort each Cargo.toml file in a workspace
```bash
cargo-sort -w/--workspace
```

These are all valid. File names and extensions can be used on some of the paths but not others, if
left off the tool will default to Cargo.toml.


```bash
cargo sort 1.0.0
Devin R <devin.ragotzy@gmail.com>
Ensure Cargo.toml dependency tables are sorted.

USAGE:
    cargo-sort [FLAGS] [CWD]

FLAGS:
    -c, --check        exit with non-zero if Cargo.toml is unsorted, overrides default behavior
    -f, --format       formats the given Cargo.toml according to tomlfmt.toml
    -g, --grouped      when sorting groups of key value pairs blank lines are kept
    -h, --help         Prints help information
    -p, --print        prints Cargo.toml, lexically sorted, to stdout
    -V, --version      Prints version information
    -w, --workspace    checks every crate in a workspace

ARGS:
    <CWD>...    sets cwd, must contain a Cargo.toml file
```

# Examples
```toml
[dependencies]
a="0.1.1"
# comments will stay with the item
c="0.1.1"

# If --grouped is used the blank line will stay.
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

# If --grouped is used the blank line will stay
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

# Tables are ordered by their appearance so
# if dev-dependencies was before build it would be
# sorted that way
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
