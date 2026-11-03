//! Headless tests driving the whole app through egui_kittest.
use crate::app::Application;
use crate::commands::Command;
use crate::events::AppEvent;
use crate::state::{AppImageData, BuildOutcome};
use egui_kittest::{Harness, kittest::Queryable};

fn harness() -> Harness<'static, Application> {
    harness_sized(1280.0, 820.0)
}

fn harness_sized(width: f32, height: f32) -> Harness<'static, Application> {
    Harness::builder()
        .with_size([width, height])
        .build_eframe(|cc| Application::create(cc, None, false))
}

fn sprite(path: &str, size: u32) -> AppImageData {
    AppImageData::new(path.to_owned(), image::DynamicImage::new_rgba8(size, size))
}

fn add(harness: &mut Harness<'static, Application>, images: Vec<AppImageData>) {
    harness.state().sender.send(AppEvent::Images(images));
    wait_built(harness);
}

/// Steps frames until the atlas matches the project.
fn wait_built(harness: &mut Harness<'static, Application>) {
    // The first frames only notice the change, so a build may not have started yet.
    for _ in 0..1000 {
        harness.step();
        let state = &harness.state().state;
        let settled = state.build.last_key == Some(state.project.pack_key())
            && state.build.pending_at.is_none()
            && !state.build.building;
        if settled {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    panic!("the build did not finish");
}

fn run(harness: &mut Harness<'static, Application>, command: Command) {
    command.schedule();
    harness.step();
    wait_built(harness);
}

fn ids(harness: &Harness<'static, Application>) -> Vec<String> {
    let images = &harness.state().state.project.image_data;
    images.iter().map(|image| image.id.clone()).collect()
}

#[test]
fn adding_images_builds_an_atlas_and_skips_duplicates() {
    let mut harness = harness();
    add(
        &mut harness,
        vec![sprite("art/a.png", 8), sprite("art/b.png", 8)],
    );
    assert_eq!(ids(&harness), ["a", "b"]);
    let state = &harness.state().state;
    let output = state.build.output.as_ref().expect("atlas built");
    assert_eq!(output.atlas_asset.frames.len(), 2);
    assert_eq!(state.build.min_size, 32);

    add(&mut harness, vec![sprite("art/a.png", 8)]);
    assert_eq!(ids(&harness).len(), 2);
}

#[test]
fn removing_an_image_renames_keys_from_the_new_common_directory() {
    let mut harness = harness();
    add(
        &mut harness,
        vec![sprite("art/x/a.png", 8), sprite("art/y/b.png", 8)],
    );
    assert_eq!(ids(&harness), ["x/a", "y/b"]);
    run(&mut harness, Command::RemoveSprite(1));
    assert_eq!(ids(&harness), ["a"]);
    let output = harness.state().state.build.output.as_ref().unwrap();
    assert_eq!(output.atlas_asset.frames[0].key, "a");
}

#[test]
fn undo_restores_images_and_rebuilds() {
    let mut harness = harness();
    add(&mut harness, vec![sprite("a.png", 8)]);
    // The undoer only records a state after the project has been stable for a while.
    harness.run_steps(20);
    run(&mut harness, Command::ClearAll);
    assert!(ids(&harness).is_empty());
    assert!(harness.state().state.build.output.is_none());

    run(&mut harness, Command::Undo);
    assert_eq!(ids(&harness), ["a"]);
    assert!(harness.state().state.build.output.is_some());
}

#[test]
fn stale_build_results_are_dropped() {
    let mut harness = harness();
    add(&mut harness, vec![sprite("a.png", 8)]);
    let generation = harness.state().state.build.generation;
    let outcome = BuildOutcome {
        min_size: 4096,
        result: Err(rpack_core::SpritesheetError::FailedToPackImage),
    };
    let tx = harness.state().sender.clone();
    tx.send(AppEvent::Build(generation - 1, outcome));
    harness.step();
    let build = &harness.state().state.build;
    assert!(build.output.is_some());
    assert!(build.last_error.is_none());
    assert_eq!(build.min_size, 32);
}

#[test]
fn renaming_patches_the_json_without_a_rebuild() {
    let mut harness = harness();
    add(&mut harness, vec![sprite("a.png", 8)]);
    let generation = harness.state().state.build.generation;
    harness.state_mut().state.project.settings.output_path = "renamed".into();
    harness
        .state_mut()
        .state
        .project
        .settings
        .skip_serializing_metadata = Some(true);
    harness.step();
    let build = &harness.state().state.build;
    let output = build.output.as_ref().unwrap();
    assert_eq!(build.generation, generation);
    assert_eq!(output.atlas_asset_json["filename"], "renamed.png");
    assert!(output.atlas_asset_json.get("metadata").is_none());
}

#[test]
fn rename_survives_a_build_that_started_before_it() {
    let mut harness = harness();
    add(&mut harness, vec![sprite("a.png", 8)]);
    harness.state_mut().state.project.settings.texture_padding = Some(4);
    // Run until the rebuild has started, then rename while it is in flight.
    for _ in 0..1000 {
        harness.step();
        if harness.state().state.build.building {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    harness.state_mut().state.project.settings.output_path = "renamed".into();
    wait_built(&mut harness);
    let output = harness.state().state.build.output.as_ref().unwrap();
    assert_eq!(output.atlas_asset.filename, "renamed.png");
}

#[test]
fn selection_supports_ranges_and_toggling() {
    let images: Vec<_> = ["a", "b", "c", "d"]
        .map(|name| sprite(&format!("{name}.png"), 2))
        .into();
    let mut ui = crate::state::UiState::default();
    let selected = |ui: &crate::state::UiState| {
        let mut paths: Vec<_> = ui.selected.iter().map(|p| p[..1].to_owned()).collect();
        paths.sort();
        paths.join("")
    };

    ui.click_sprite(1, egui::Modifiers::NONE, &images);
    assert_eq!(selected(&ui), "b");
    ui.click_sprite(3, egui::Modifiers::SHIFT, &images);
    assert_eq!(selected(&ui), "bcd");
    ui.click_sprite(0, egui::Modifiers::COMMAND, &images);
    assert_eq!(selected(&ui), "abcd");
    ui.click_sprite(0, egui::Modifiers::COMMAND, &images);
    assert_eq!(selected(&ui), "bcd");

    ui.click_sprite(0, egui::Modifiers::NONE, &images);
    ui.click_sprite(1, egui::Modifiers::SHIFT, &images);
    assert_eq!(selected(&ui), "ab");
    ui.selection_anchor = Some(3);
    ui.click_sprite(
        3,
        egui::Modifiers::SHIFT | egui::Modifiers::COMMAND,
        &images,
    );
    assert_eq!(selected(&ui), "abd");

    ui.prune_selection(&images[..2]);
    assert_eq!(selected(&ui), "ab");
    assert_eq!(ui.selection_anchor, None);
}

#[test]
fn shortcuts_do_not_run_disabled_commands() {
    let mut harness = harness();
    Command::RemoveSelected.schedule();
    Command::Undo.schedule();
    harness.step();
    assert!(harness.state().state.project.image_data.is_empty());
}

#[test]
fn toolbar_buttons_run_their_commands() {
    let mut harness = harness();
    add(&mut harness, vec![sprite("a.png", 8)]);

    let outlines = harness.state().state.ui.settings.show_outlines;
    harness.get_by_label("Sprite outlines").click();
    harness.run_steps(2);
    assert_ne!(harness.state().state.ui.settings.show_outlines, outlines);

    harness.get_by_label("Sprites panel").click();
    harness.run_steps(2);
    assert!(!harness.state().state.ui.settings.show_sprites);
}

#[test]
fn clicking_a_sprite_selects_it_and_remove_drops_it() {
    let mut harness = harness();
    add(
        &mut harness,
        vec![sprite("art/a.png", 8), sprite("art/b.png", 8)],
    );
    harness.get_by_label("b").click();
    harness.run_steps(2);
    let selected = &harness.state().state.ui.selected;
    assert!(selected.contains("art/b.png") && selected.len() == 1);

    harness.get_by_label("Remove").click();
    harness.step();
    wait_built(&mut harness);
    assert_eq!(ids(&harness), ["a"]);
}

#[test]
fn hovering_the_atlas_highlights_without_outlines() {
    let mut harness = harness();
    // A sprite that fills the atlas, so the middle of the preview is on it.
    harness.state_mut().state.project.settings.size = Some(64);
    add(&mut harness, vec![sprite("a.png", 64)]);
    assert!(!harness.state().state.ui.settings.show_outlines);
    harness.hover_at(egui::pos2(620.0, 330.0));
    harness.run_steps(3);
    assert_eq!(harness.state().state.ui.hovered_frame.as_deref(), Some("a"));
}

#[test]
fn inspector_hides_on_a_narrow_window_and_can_be_forced_open() {
    let mut harness = harness_sized(900.0, 600.0);
    harness.run_steps(2);
    assert!(!harness.state().state.ui.inspector_visible());
    assert!(harness.state().state.ui.settings.show_inspector);

    harness.get_by_label("Inspector").click();
    harness.run_steps(2);
    assert!(harness.state().state.ui.inspector_visible());

    harness.set_size(egui::vec2(1280.0, 820.0));
    harness.run_steps(2);
    assert!(harness.state().state.ui.inspector_visible());
}

#[test]
fn settings_are_reachable_before_any_image_is_added() {
    let harness = harness();
    harness.get_by_label("File name");
    harness.get_by_label("Skip metadata");
}

#[test]
fn start_screen_offers_adding_images() {
    let harness = harness();
    harness.get_by_label("Add images...");
    assert_eq!(harness.get_all_by_label("No images yet").count(), 2);
}

#[test]
fn about_opens_and_closes() {
    let mut harness = harness();
    run(&mut harness, Command::ShowAbout);
    harness.run_steps(2);
    harness.get_by_label("Close").click();
    harness.run_steps(3);
    assert!(harness.state().state.ui.dialog.is_none());
}

#[test]
fn a_config_size_outside_the_list_can_be_reselected() {
    let mut harness = harness();
    harness.state_mut().state.project.settings.size = Some(300);
    harness.run_steps(2);
    harness.get_by_label("300×300").click();
    harness.run_steps(2);
    harness
        .get_all_by_label("300×300")
        .next()
        .expect("the current size stays in the menu");
}

#[test]
fn native_menu_replaces_the_egui_one_and_keeps_its_shortcuts_out() {
    let mut harness = harness();
    add(&mut harness, vec![sprite("art/a.png", 8)]);
    assert!(harness.query_by_label("File").is_some());

    let ui = &mut harness.state_mut().state.ui;
    ui.native_chrome = true;
    ui.native_menu = true;
    harness.step();
    assert!(harness.query_by_label("File").is_none());

    // AppKit handles Cmd+A as a key equivalent. Reaching egui, it must not run twice.
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.step();
    assert!(harness.state().state.ui.selected.is_empty());
    // Delete has no key equivalent, so it stays with egui.
    run(&mut harness, Command::SelectAll);
    harness.key_press(egui::Key::Delete);
    harness.step();
    assert!(ids(&harness).is_empty());
}

mod snapshots {
    use super::*;

    /// Paths are relative so the Path row in the snapshots does not depend on the machine.
    const TILES: [&str; 6] = [
        "../bevy_rpack/tiles/agents/spaceAstronauts_004.png",
        "../bevy_rpack/tiles/agents/spaceShips_003.png",
        "../bevy_rpack/tiles/effects/spaceEffects_008.png",
        "../bevy_rpack/tiles/missiles/spaceMissiles_010.png",
        "../bevy_rpack/tiles/missiles/spaceMissiles_014.png",
        "../bevy_rpack/tiles/ship/spaceBuilding_001.png",
    ];

    fn loaded(width: f32, height: f32) -> Harness<'static, Application> {
        let mut harness = harness_sized(width, height);
        harness.state_mut().state.project.settings.size = Some(256);
        let images = TILES
            .iter()
            .map(|path| AppImageData::from_file(path.as_ref()).expect("fixture image"))
            .collect();
        add(&mut harness, images);
        settle(&mut harness);
        harness
    }

    /// Image loaders work in the background, so give them a moment before drawing.
    fn settle(harness: &mut Harness<'static, Application>) {
        for _ in 0..10 {
            harness.step();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    #[test]
    fn start_screen() {
        let mut harness = harness();
        settle(&mut harness);
        harness.snapshot("start_screen");
    }

    #[test]
    fn atlas_loaded() {
        let mut harness = loaded(1280.0, 820.0);
        harness.snapshot("atlas_loaded");
    }

    #[test]
    fn selection_and_outlines() {
        let mut harness = loaded(1280.0, 820.0);
        harness.get_by_label("agents/spaceShips_003").click();
        run(&mut harness, Command::ToggleOutlines);
        settle(&mut harness);
        harness.snapshot("selection");
    }

    #[test]
    fn json_hidden() {
        let mut harness = loaded(1280.0, 820.0);
        run(&mut harness, Command::ToggleJson);
        settle(&mut harness);
        harness.snapshot("json_hidden");
    }

    #[test]
    fn narrow_window() {
        let mut harness = loaded(800.0, 560.0);
        harness.snapshot("narrow_window");
    }

    #[test]
    fn about_sheet() {
        let mut harness = harness();
        run(&mut harness, Command::ShowAbout);
        settle(&mut harness);
        harness.snapshot("about");
    }

    /// The macOS layout at the minimum width, where the toolbar is the tightest.
    #[test]
    fn native_chrome() {
        let mut harness = loaded(720.0, 460.0);
        let ui = &mut harness.state_mut().state.ui;
        ui.native_chrome = true;
        ui.native_menu = true;
        settle(&mut harness);
        harness.snapshot("native_chrome");
    }

    #[test]
    fn toast_sits_above_the_status_bar() {
        let mut harness = harness();
        let tx = harness.state().sender.clone();
        tx.notify(crate::ui::toasts::Level::Error, "Could not read broken.png");
        harness.step();
        harness.step();
        harness.snapshot("toast");
    }
}

/// `theme/colors.rs` is the one place for colors. Tests may use their own.
#[test]
fn colors_live_in_the_theme_file() {
    fn visit(dir: &std::path::Path, stray: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(&path, stray);
            } else if !path.ends_with("theme/colors.rs") && !path.ends_with("tests/mod.rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                let code = text.split("#[cfg(test)]").next().unwrap_or_default();
                for line in code.lines().filter(|line| line.contains("Color32::")) {
                    stray.push(format!("{}: {}", path.display(), line.trim()));
                }
            }
        }
    }
    let mut stray = Vec::new();
    visit(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src")),
        &mut stray,
    );
    assert!(stray.is_empty(), "colors outside the theme: {stray:#?}");
}
