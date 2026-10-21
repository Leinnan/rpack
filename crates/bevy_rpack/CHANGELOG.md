# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0]

### Added

- Share atlas packing, dimension-only fit checks, configuration defaults, and frame naming through `bevy_rpack::packing`, available without Bevy.
- `RpackAtlasAssetLoader` loader responsible for loading `RpackAtlasAsset` files from `.rpack.json` files.

### Changed

- Updated to Bevy 0.20
- CLI generation now reserves borders correctly and accepts exact fits. Frame keys remove only the final extension; dotted names may produce different keys than older CLI versions.
- CLI generation rejects invalid configuration (including sizes above 8192), duplicate keys, empty sources, invalid glob patterns, and decode failures. Overlapping patterns are deduplicated.
- Preserve advanced CLI packing through the upstream exporter. Deprecate the legacy CLI skyline module; the GUI uses shared placement instead.

## [0.5.0]

### Changed

- Updated to Bevy 0.19

## [0.4.0]

### Changed

- Updated to Bevy 0.18

## [0.3.1]

### Fixed

- Exposed `RpackAtlasAssetLoaderSettings`


## [0.3.0]

### Added

- Introduced `metadata` field in tilemap format.

### Changed

- Updated to Bevy 0.17

## [0.2.0] - 2025-05-07

### Added

- `RpackAtlasAssetLoaderSettings` for asset loading

### Changed

- Updated to Bevy 0.16

## [0.1.2] - 2025-01-27

### Added

- more documentation

## [0.1.1] - 2025-01-27

### Added

- `RpackAtlases` system param.

## [0.1.0] - 2025-01-14

- initial version
