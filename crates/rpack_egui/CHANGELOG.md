# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Dropping a folder also adds the images in its subfolders. A folder holding only subfolders used to add nothing, without any message.
- Messages shown after a file dialog, such as "Saved atlas.png", no longer disappear early. The time the dialog was open used to count against them.

## [0.7.0]

### Added

- About sheet with the version, author and homepage.
- Inspector with collapsible sections, including details of the selected sprite.
- Open config command, zoom presets, and buttons and menu entries to show or hide the side panels.

### Changed

- **New layout:** menu bar, toolbar, sprites on the left, inspector on the right, and the atlas JSON under the preview.
- **New theme:** dark only, so the theme buttons are gone. Lucide icons replace Material icons.
- macOS: native menu bar, and the toolbar shares its row with the window buttons.
- New app icon: sprites packed into an atlas. The SVG sources are in `assets/icon/`.
- Undo and Redo moved to the toolbar, and the author link and version moved to the About sheet.
- Interface scale now scales all of the UI and starts at 1.0. Saved recent projects are kept; saved view options are not.
- Settings can be changed before adding images, and the sprite list stays usable during a rebuild.
- Selection highlight and syncing the list with the hovered sprite no longer need outlines.
- Only one build runs at a time, and it no longer copies the source images.
- "Auto (up to 2048)" is now "Auto (2048, grows to fit)", which is what it does. Output is unchanged.
- Copy JSON gives the same pretty form as the panel and the saved file.
- Depends on `rpack_core` instead of `rpack_cli`.

### Fixed

- Renaming during a build no longer brings back the old name.
- A config that fails to load no longer clears the current project.
- A config given on the command line is added to the recent projects.
- Sizes from a config that are not in the menu can be selected again.
- The web drop overlay no longer promises folders or configs.
- A failed web start now reports an error.
- Installing from crates.io no longer needs git history.

### Removed

- Unused PWA manifest and service worker.

## [0.6.0]

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
