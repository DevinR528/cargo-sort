# 2.0.1

Bug fixes

* Fix unintended merging of dependency groups with `--grouped` when dotted key syntax was used

# 2.0.0

This release is a big one! Special thanks go out to @thomaseizinger who fixed a lot of regressions
from the initial `toml_edit` upgrade.
Thanks also to @ssrlive for contributing a bunch of improvements.

Highlights

* Upgrade `toml_edit` to support more TOML syntax (in particular, `something.workspace = true`)
* Format `Cargo.toml` files by default
  * Use `-n` or `--no-format` to disable this
  * `--check` mode also verifies formatting unless you pass the abovementioned flag
* `workspace.dependencies` (and `build-` / `dev-` dependencies) are now also sorted
* `target.'cfg(something)'.dependencies` now go right after `dependencies`
  * The same goes for target-specific `dev-dependencies`

Other Improvements

* Remove unused dependencies
* In non-check mode, report whether files were already sorted or not

# 1.1.0

Yanked, because it did invalid changes in many situations.

# 1.0.9

Bug Fixes

  * The `--workspace` feature now respects the exclude array


# 1.0.8

Update

  * Update clap from 2.34 to 4.0.10

Feature

  * Add --check-format flag
    * If set, `cargo-sort` will check formatting (allows only checking formatting)
    * [Thanks matze](https://github.com/DevinR528/cargo-sort/pull/41)
  * DockerHub builds added
    * [Thanks orhun](https://github.com/DevinR528/cargo-sort/pull/44)



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
