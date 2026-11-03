//! Main actions on the left, the project name in the middle, view options on the right.
use super::theme::colors::{BORDER, CARD_BG, CARD_BG_HOVER, TEXT};
use super::widgets::buttons::{Tip, toolbar_icon, toolbar_toggle};
use crate::commands::Command;
use crate::state::AppState;
use egui::{
    Align, Button, Layout, Rect, RichText, Sense, Stroke, TextEdit, Ui, UiBuilder, ViewportCommand,
    containers::menu::MenuButton, vec2,
};
use lucide_icons::Icon;

pub const TOOLBAR_HEIGHT: f32 = 38.0;
/// Room for the macOS traffic lights, which don't follow the interface scale.
const TRAFFIC_LIGHT_INSET: f32 = 76.0;
const NAME_WIDTH: f32 = 200.0;
const ZOOM_PRESETS: [u16; 6] = [25, 50, 100, 200, 400, 800];

fn action(ui: &mut Ui, state: &AppState, icon: Icon, name: &str, command: Command) {
    let tip = Tip::command(name, command);
    if toolbar_icon(ui, icon, command.enabled(state), tip).clicked() {
        command.schedule();
    }
}

fn toggle(ui: &mut Ui, icon: Icon, on: bool, name: &str, command: Command) {
    if toolbar_toggle(ui, icon, on, Tip::command(name, command)).clicked() {
        command.schedule();
    }
}

/// Where the bar's first control starts. Full screen hides the traffic lights.
pub fn leading_inset(ui: &Ui, state: &AppState) -> f32 {
    let fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
    if state.ui.native_chrome && !fullscreen {
        TRAFFIC_LIGHT_INSET / ui.ctx().zoom_factor()
    } else {
        8.0
    }
}

/// With no title bar, the toolbar's empty space moves the window and a double click
/// zooms it. Called before the controls are added, so they keep priority.
fn handle_window_drag(ui: &Ui, rect: Rect) {
    let response = ui.interact(
        rect,
        ui.scope_id().with("window_drag"),
        Sense::click_and_drag(),
    );
    if response.drag_started() {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if response.double_clicked() {
        let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState) {
    profile_scope!("toolbar");
    let full = ui.max_rect();
    if state.ui.native_chrome {
        handle_window_drag(ui, full);
    }
    ui.spacing_mut().item_spacing.x = 4.0;
    let inset = leading_inset(ui, state);
    let left_end = ui
        .horizontal_centered(|ui| {
            ui.add_space(inset);
            action(ui, state, Icon::ImagePlus, "Add images", Command::AddImages);
            #[cfg(not(target_arch = "wasm32"))]
            action(
                ui,
                state,
                Icon::FolderOpen,
                "Open config",
                Command::OpenConfig,
            );
            ui.separator();
            action(ui, state, Icon::Save, "Save image", Command::SaveAtlas);
            action(ui, state, Icon::Braces, "Save JSON", Command::SaveJson);
            ui.separator();
            action(ui, state, Icon::Undo2, "Undo", Command::Undo);
            action(ui, state, Icon::Redo2, "Redo", Command::Redo);
        })
        .response
        .rect
        .right();
    let right = ui.new_child(
        UiBuilder::new()
            .max_rect(full)
            .layout(Layout::right_to_left(Align::Center)),
    );
    let right_start = view_options(right, state);
    let name_rect = Rect::from_center_size(full.center(), vec2(NAME_WIDTH, 24.0));
    // Below this width the name would overlap the buttons. The inspector still has it.
    if name_rect.left() > left_end + 8.0 && name_rect.right() < right_start - 8.0 {
        let mut name = ui.new_child(UiBuilder::new().max_rect(name_rect));
        project_name(&mut name, state);
    }
}

/// Returns where the options start.
fn view_options(mut ui: Ui, state: &AppState) -> f32 {
    let ui = &mut ui;
    ui.spacing_mut().item_spacing.x = 4.0;
    ui.add_space(8.0);
    let view = &state.ui.settings;
    toggle(
        ui,
        Icon::PanelRight,
        state.ui.inspector_visible(),
        "Inspector",
        Command::ToggleInspector,
    );
    toggle(
        ui,
        Icon::PanelLeft,
        view.show_sprites,
        "Sprites panel",
        Command::ToggleSprites,
    );
    ui.separator();
    toggle(
        ui,
        Icon::Braces,
        view.display_json,
        "Atlas JSON",
        Command::ToggleJson,
    );
    toggle(
        ui,
        Icon::Grid2x2,
        view.show_outlines,
        "Sprite outlines",
        Command::ToggleOutlines,
    );
    ui.separator();
    zoom_menu(ui, state);
    action(ui, state, Icon::ZoomIn, "Actual size", Command::ActualSize);
    action(ui, state, Icon::Maximize, "Fit", Command::FitPreview);
    ui.min_rect().left()
}

fn zoom_menu(ui: &mut Ui, state: &AppState) {
    let has_atlas = state.build.output.is_some();
    let label = format!("{}%", state.ui.preview.zoom_percent());
    ui.add_enabled_ui(has_atlas, |ui| {
        MenuButton::from_button(Button::new(label).min_size(vec2(56.0, 24.0))).ui(ui, |ui| {
            for percent in ZOOM_PRESETS {
                if ui.button(format!("{percent}%")).clicked() {
                    Command::SetZoom(percent).schedule();
                    ui.close();
                }
            }
        });
    });
}

/// A chip that turns into a text field on click.
fn project_name(ui: &mut Ui, state: &mut AppState) {
    let output_path = &mut state.project.settings.output_path;
    if state.ui.renaming {
        let response = ui.add_sized(
            ui.available_size(),
            TextEdit::singleline(output_path).hint_text("File name"),
        );
        response.request_focus();
        let submitted =
            ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape));
        let clicked_away = ui.input(|i| i.pointer.any_pressed()) && !response.hovered();
        if submitted || clicked_away {
            state.ui.renaming = false;
        }
        return;
    }
    let chip = Button::new(RichText::new(output_path.as_str()).color(TEXT))
        .fill(CARD_BG)
        .stroke(Stroke::new(1.0, BORDER))
        .truncate();
    let response = ui
        .add_sized(ui.available_size(), chip)
        .on_hover_text("File name, click to change");
    if response.clicked() {
        state.ui.renaming = true;
    }
    if response.hovered() {
        ui.painter().rect_stroke(
            response.rect,
            6.0,
            Stroke::new(1.0, CARD_BG_HOVER),
            egui::StrokeKind::Inside,
        );
    }
}
