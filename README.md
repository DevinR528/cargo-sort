# Cargo Sort Check

[![Build Status](https://travis-ci.com/DevinR528/cargo-sort-ck.svg?branch=master)](https://travis-ci.com/DevinR528/cargo-sort-ck)
[![Latest Version](https://img.shields.io/crates/v/cargo-sort-ck.svg)](https://crates.io/crates/toml)

A tool to check that your Cargo.toml dependencies are sorted alphabetically. Project inspired by
[jpoles1](https://github.com/jpoles1) as a solution to @dtolnay's [request for implementation #29](https://github.com/dtolnay/request-for-implementation/issues/29).  Cross platform implementation, windows compatible.  Checks/sorts by key in tables and also nested table header (does not sort the items in a nested header). To pass the nested tables must be grouped. 

[toml]: https://github.com/toml-lang/toml
included in sort check is:
```toml
["dependencies"]
["dev-dependencies"]
["build-dependencies"]
["workspace.members"]
["workspace.exclude"]
```

# Install
```bash
cargo install cargo-sort-ck
```

# Run
Defaults to current dir but any path can be passed in 
```bash
cargo-sort-ck [cwd]
cargo-sort-ck [path/to/dir | ./path/to/dir | path/to/Cargo.toml]
```

# Examples
```toml
[dependencies]
a="0.1.1"
c="0.1.1"
b="0.1.1"

[dependencies.alpha]
version="0"

[build-dependencies]
foo="0"
bar="0"

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
b="0.1.1"
c="0.1.1"

[dependencies.alpha]
version="0"

[dependencies.beta]
version="0"

[dependencies.zed]
version="0"

[build-dependencies]
bar="0"
foo="0"

[dev-dependencies]
bar="0"
foo="0"

```