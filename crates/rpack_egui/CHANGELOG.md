# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- File / Edit / View menus, keyboard shortcuts (open, save, undo, redo, select all, delete) and an always visible "Add images" button.
- Notifications for saves, copies and load errors instead of silent failures.
- Drop overlay while dragging files over the window.
- Atlas preview with zoom, pan, transparency checkerboard and sprite outlines with names on hover.
- Multi-selection in the image list, a remove button per row and duplicate detection.
- Saving the current inputs and settings as a `.rpack_gen.json` config.
- Atlas statistics in the status bar, project name in the window title.
- Adjustable interface scale, Material icons, screen reader support (native).

### Changed

- Updated to egui 0.37.
- The previous atlas stays visible while a new one is being built; slider changes are debounced.
- Image decoding, minimum size search and PNG encoding no longer block the UI thread.
- Undo snapshots no longer copy or compare pixel data.
- Larger default window, recent projects are limited to 8, moved to the front when reopened and pruned when missing.

### Fixed

- Results of finished background work were not shown until the next input event.
- An older build could overwrite the result of a newer one.
- Removing an image could remove the wrong one or panic.
- Failed saves could panic; the semibold font could be missing.
- Atlas json filename lost its `.png` extension after renaming.
