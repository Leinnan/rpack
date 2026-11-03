# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- `sources::expand_directories` also expands subdirectories, without following symlinked ones.

## [0.7.0]

### Added

- First release. Holds the atlas types and `packing` from `bevy_rpack`, and `Spritesheet`, `TilemapGenerationConfig` and `SaveImageFormat` from `rpack_cli`. Both crates re-export them at their old paths.
- `pipeline` finds the smallest fitting atlas size and builds from borrowed images.
- `sources` expands dropped folders and escapes paths for glob patterns.
- `TilemapGenerationConfig::from_json` and `to_project_json` read and write configs without touching the file system.
