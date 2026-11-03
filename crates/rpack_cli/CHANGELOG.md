# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0]

### Changed

- **Breaking:** `TilemapGenerationConfig::generate` is now provided by the `GenerateAtlas` trait, so callers need `use rpack_cli::GenerateAtlas;`.
- `Spritesheet`, `TilemapGenerationConfig`, `SaveImageFormat` and related types now live in the new `rpack_core` crate. They are re-exported here at the same paths.
- The `rpack_cli` binary now requires the `cli` feature, so `--no-default-features` builds of the library work.

### Fixed

- Saving DDS without the `basis` feature no longer returns an error after writing the file, and saving DDS with `dds` disabled now reports an error instead of doing nothing.

## [0.1.2] - 2025-12-19

### Added

- Resize option during conversion

## [0.1.1] - 2025-01-27

### Added

- When no arguments is provided program will look for config files in current directory and generate a tilemap if there is only one config file.
- Better logs.

## [0.1.0] - 2025-01-14

- initial version
