# Cargo Sort

[![Crates.io](https://img.shields.io/crates/v/cargo-sort.svg)](https://crates.io/crates/cargo-sort)
[![Rust Stable](https://github.com/DevinR528/cargo-sort-ck/actions/workflows/stable.yml/badge.svg)](https://github.com/DevinR528/cargo-sort-ck/actions/workflows/stable.yml)

A tool to check that your Cargo.toml dependencies are sorted alphabetically. Project created as a solution to @dtolnay's [request for implementation #29](https://github.com/dtolnay/request-for-implementation/issues/29). Cross platform implementation, windows compatible.  Terminal coloring works on both cmd.exe and powershell. Checks/sorts by key in tables and also nested table headers (does not sort the items in a nested header, sorts the table itself). `cargo sort` uses [toml-edit](https://github.com/ordian/toml_edit) to parse the toml file into something useful.

The `--format` option may result in improperly formatted toml; please file an issue.

## Use

There are three modes cargo-sort can be used in:
 * **default**
    - No flags set cargo-sort will write the sorted result over the input Cargo.toml file.
 * **-c or --check**
    - Will fail with a non-zero exit code if the file is unsorted.
 * **-n or --no-format**
    - Will **NOT** format the sorted toml. This option only has an effect if writing or printing out.
 * **--check-format**
    - Checks that after sorting the original input file has not changed.
 * **-g or --grouped**
    - When sorting keep table key value spacing. If you have dependency groups they will stick but be sorted within the grouping.
    The `key_value_newlines` config option needs to be `true` for this to have any effect.
 * **-p or --print**
    - Write the sorted toml file to stdout.
 * **-w or --workspace**
    - Checks every crate in the workspace based on flags. Only one root may be given.
 * **-o or --order**
    - Specify an ordering of tables. All nested tables will be sorted and appear after the specified table. Any unspecified table will be after specified.
 * **--config**
    - Specify a custom path to the `tomlfmt.toml` configuration file. Useful for projects that organize config files in a dedicated directory (e.g., `.config/tomlfmt.toml`).

### Config

`cargo sort` uses a config file when formatting called `tomlfmt.toml`. This is optional and defaults will
be used if not found in the current working dir. You can also specify a custom config path using the
`--config` flag:

```bash
cargo sort --config .config/tomlfmt.toml
```

Here are the defaults when no `tomlfmt.toml` is found
```toml
# trailing comma in arrays
always_trailing_comma = false
# trailing comma when multi-line
multiline_trailing_comma = true
# the maximum length in bytes of the string of an array object
max_array_line_len = 80
# number of spaces to indent
indent_count = 4
# space around equal sign
space_around_eq = true
# remove all the spacing inside the array
compact_arrays = false
# remove all the spacing inside the object
compact_inline_tables = false
trailing_newline = true
# is it ok to have blank lines inside of a table
# this option needs to be true for the --grouped flag
key_value_newlines = true
allowed_blank_lines = 1
# windows style line endings
crlf = false
# The user specified ordering of tables in a document.
# All unspecified tables will come after these.
table_order = [
    "package",
    "workspace",
    "lib",
    "bin",
    "features",
    "dependencies",
    "build-dependencies",
    "dev-dependencies",
]
```

included in sort check is:
```toml
["dependencies"]
["dev-dependencies"]
["build-dependencies"]
["workspace.members"]
["workspace.exclude"]
```

If you have a header to add open a PR, they are welcome.


# Install
```bash
cargo install cargo-sort
```

## pre-commit

If you use [pre-commit](https://pre-commit.com/) in your project, you can add cargo-sort as hook by
adding the following entry to your `.pre-commit-config.yaml` configuration:

```yaml
repos:
- repo: https://github.com/DevinR528/cargo-sort
  rev: v1.0.4
  hooks:
  - id: cargo-sort
```

Please make sure to set `rev` to the latest tag of this repo as the tag shown here might not always
be updated to the latest version.

# Run

Thanks to [dspicher](https://github.com/dspicher) for [issue #4](https://github.com/DevinR528/cargo-sort-ck/issues/4) you can now invoke `cargo sort` check as a cargo subcommand

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
Ensure Cargo.toml dependency tables are sorted

Usage: cargo sort [OPTIONS] [CWD]...

Arguments:
  [CWD]...  sets cwd, must contain a Cargo.toml file

Options:
  -c, --check          Returns non-zero exit code if Cargo.toml is unsorted
  -p, --print          Prints Cargo.toml, lexically sorted, to stdout
  -n, --no-format      Skips formatting after sorting
      --check-format   Also returns non-zero exit code if formatting changes
  -w, --workspace      Checks every crate in a workspace
  -g, --grouped        Keep blank lines when sorting groups of key value pairs
  -o, --order <ORDER>  List the order tables should be written out
      --config <PATH>  Path to a custom config file (tomlfmt.toml)
  -h, --help           Print help
  -V, --version        Print version
```

# Docker

Build the image:

```sh
docker build -t cargo-sort .
```

Run the container:

```sh
docker run -it --rm -v "$(pwd)/Cargo.toml":/app/Cargo.toml cargo-sort
```

Image is also available on [Docker Hub](https://hub.docker.com/r/devinr528/cargo-sort):

```sh
docker run -it --rm -v "$(pwd)/Cargo.toml":/app/Cargo.toml devinr528/cargo-sort:latest
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
# sorted that way unless --order is specified
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
