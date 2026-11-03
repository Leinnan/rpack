# rPack task runner. `just` lists the recipes.

default:
    @just --list

# Run the desktop app; an extra argument is a config to open
[group('dev')]
run *args:
    cargo run -p rpack_egui -- {{args}}

# Serve the web build at http://localhost:8080, rebuilding on changes
[group('dev')]
web:
    cd crates/rpack_egui && trunk serve

# Check that the app builds for the web
[group('code')]
check-wasm:
    RUSTFLAGS="--cfg=web_sys_unstable_apis" cargo check -p rpack_egui --target wasm32-unknown-unknown

# Format check, wasm check, icon check, clippy and tests (what CI runs)
[group('code')]
validate: check-wasm icons-check
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

# Re-record the egui_kittest snapshot images
[group('code')]
snapshots:
    UPDATE_SNAPSHOTS=1 cargo test -p rpack_egui

# Re-record the expected packing output (only when an output change is intended)
[group('code')]
golden-update:
    RPACK_UPDATE_GOLDEN=1 cargo test -p rpack_cli --test golden

# Render the app icon SVGs to the PNG and favicon the app ships
[group('assets')]
icons:
    cargo xtask icons

# Regenerate the icon font subset (the icons the app actually uses)
[group('assets')]
icons-subset:
    cargo xtask icons-subset

# Fail when the icon font subset is stale
[group('assets')]
icons-check:
    cargo xtask icons-subset --check

# Replace the README screenshot with one of the real app
[group('assets')]
screenshot:
    cargo build -p rpack_egui
    RPACK_SCREENSHOT=docs/media/screenshot-atlas.png target/debug/rpack_egui crates/bevy_rpack/example_config.rpack_gen.json

# Create the desktop bundle with cargo-bundler
[group('package')]
bundle:
    cd crates/rpack_egui && just bundle
