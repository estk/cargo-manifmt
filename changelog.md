# Changelog

## [Unreleased]

## [2.0.0]

### Changed

* name from cargo-sort to cargo-manifmt
* clippy fixes @orhun
* check format flag @matze
* better help texts @matze
* update clap @DevinR528
* update toml_edit @matze
* Sort by workspace level @estk
* Refactor main @estk
* Better dependency group_by @estk
* Newlines between groups @estk
* fix CI release & changelog

### Added

* workspace dependencies @dspicher
* docker support + docs @orhun


## [1.0.9]

### Fixed

  * The `--workspace` feature now respects the exclude array


## [1.0.8]

### Changed

  * Update clap from 2.34 to 4.0.10

### Added

  *  Add --check-format flag
    * If set, `cargo-sort` will check formatting (allows only checking formatting)
    * [Thanks matze](https://github.com/DevinR528/cargo-sort/pull/41)
  * DockerHub builds added
    * [Thanks orhun](https://github.com/DevinR528/cargo-sort/pull/44)



## [1.0.7]

### Fixed

  * Fix leaving files in the list of paths to check when `--workspace` is used with globs
    * [Thanks innuwa](https://github.com/DevinR528/cargo-sort/issues/33)
  * Fix the cargo install always re-installing https://github.com/rust-lang/cargo/issues/8703

## [1.0.6]

### Fixed

  * Fix handling of windows style line endings
    * [Thanks jose-acevedoflores](https://github.com/DevinR528/cargo-sort/pull/28)

## [1.0.5]

### Added

  * Add colorized help output
    * [Thanks QuarticCat](https://github.com/DevinR528/cargo-sort/pull/21)

## [1.0.4]

### Fixed

  * Fix trailing comma in multi-line arrays

## [1.0.3]

  * Simplify output of running cargo-sort
  * Add `--order` flag to specify ordering of top-level tables

## [1.0.2]

### Changed

  * Remove toml-parse crate in favor of toml_edit
  * Changed name from cargo-sort-ck to cargo-sort
