# Cargo Sort Check

[![Build Status](https://travis-ci.com/DevinR528/cargo-sort-ck.svg?branch=master)](https://travis-ci.com/DevinR528/cargo-sort-ck)
[![Latest Version](https://img.shields.io/crates/v/cargo-sort-ck.svg)](https://crates.io/crates/toml)
[![Documentation](https://docs.rs/cargo-sort-ck/badge.svg)](https://docs.rs/toml)

A simple tool to check for a sorted Cargo.toml file

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
```