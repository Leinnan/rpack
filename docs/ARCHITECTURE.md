# Architecture

## Crates

```
rpack_core <- bevy_rpack    Bevy plugin and asset loaders
rpack_core <- rpack_cli     clap binary, DDS and Basis saving
rpack_core <- rpack_egui    desktop and web GUI
```

- `rpack_core` knows nothing about UIs. It depends on Bevy and clap only through optional features, which exist because their derives must sit on the type itself.
- `bevy_rpack` and `rpack_cli` re-export what moved to core at the old paths, so their public paths stay valid.
- Versions move together. Publish in the order `rpack_core`, `bevy_rpack`, `rpack_cli`, `rpack_egui`.
- `rpack_cli/tests/golden.rs` holds the expected output of the example config. It should only change when a change to packing or the JSON format is intended.

These must stay stable: crate names, the `rpack_cli` binary name, the `.rpack.json` and `.rpack_gen.json` extensions and their field names, the Bevy labels `#atlas_texture` and `#atlas_layout`, the bundle id and the `rPack` app name, which decides where settings are stored.

## rpack_egui

Each frame runs `logic`, then `ui`:

1. `logic` (`app/mod.rs`) handles worker events and shortcuts, applies commands and starts builds.
2. `ui` (`ui/mod.rs`) draws the panels from `AppState`, and then applies any commands they scheduled.

The rules that keep this simple:

- **Panels do no work.** They render, schedule a `Command`, or edit plain state such as settings fields. File IO and packing never happen while rendering.
- **The menu table is the source of truth.** `menu.rs` drives the menu bar, shortcut dispatch and tooltips. `Command::enabled` decides what is available, for menus and buttons alike.
- **macOS gets native chrome.** `platform/macos_menu.rs` builds the menu bar from the same table, and the toolbar doubles as the title bar. Tests run without it, so the egui menu stays covered on every OS.
- **Workers only send events.** Background work reports through `EventSender`. `platform/` hides how that runs on each target: threads on native, the main thread's task queue on the web.
- **Builds follow the project.** `app/build.rs` compares `Project::pack_key` each frame, so any change to sprites or layout settings rebuilds, whatever caused it, including undo. The file name and metadata flag are patched into the output instead.
- **Colors live in `ui/theme/colors.rs`.** A test enforces it. Text never sits directly on `ACCENT`, as the contrast is too low; use `ACCENT_FILL`.
- **Icon-only buttons have an accessible name.** `Tip` provides it. Tests find widgets by that name, and kittest fails on widgets without one.

## Testing

- `just validate` runs what CI runs.
- `src/tests/` drives the whole app through `egui_kittest`.
- Snapshots are recorded on macOS with egui's default font and compared only there. `just snapshots` re-records them.
- `RPACK_SCREENSHOT=out.png` makes a debug build capture the real window and quit.
- After adding an `Icon::…`, run `just icons-subset`, or it renders as a blank box and CI fails.
