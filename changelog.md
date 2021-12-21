# 1.0.7

Bug Fixes

  * Fix leaving files in the list of paths to check when `--workspace` is used with globs
    * [Thanks innuwa](https://github.com/DevinR528/cargo-sort/issues/33)
  * Fix the cargo install always re-installing https://github.com/rust-lang/cargo/issues/8703
# 1.0.6

Bug Fixes

  * Fix handling of windows style line endings
    * [Thanks jose-acevedoflores](https://github.com/DevinR528/cargo-sort/pull/28)

# 1.0.5

Feature

  * Add colorized help output
    * [Thanks QuarticCat](https://github.com/DevinR528/cargo-sort/pull/21)

# 1.0.4

Bug Fixes

  * Fix trailing comma in multi-line arrays

# 1.0.3

  * Simplify output of running cargo-sort
  * Add `--order` flag to specify ordering of top-level tables

# 1.0.2

Overhaul

  * Remove toml-parse crate in favor of toml_edit
  * Changed name from cargo-sort-ck to cargo-sort
