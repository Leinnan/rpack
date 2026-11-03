//! Shown while the project has no images.
use super::theme::colors::{HINT_TEXT, TEXT_WEAK};
use super::widgets::buttons::primary_button;
use crate::commands::Command;
use crate::state::AppState;
use egui::{RichText, Ui};
use lucide_icons::Icon;

#[cfg(not(target_arch = "wasm32"))]
const DROP_HINT: &str = "Drop images, a folder or a config here";
#[cfg(target_arch = "wasm32")]
const DROP_HINT: &str = "Drop images here";

pub fn show(ui: &mut Ui, state: &AppState) {
    egui::ScrollArea::vertical()
        .id_salt("start_screen")
        .auto_shrink(false)
        .show(ui, |ui| {
            let rows = recent_rows(ui, state);
            let height = 170.0 + rows as f32 * 96.0;
            ui.add_space(((ui.available_height() - height) / 2.0).max(24.0));
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(char::from(Icon::Images))
                        .size(44.0)
                        .color(HINT_TEXT),
                );
                ui.add_space(6.0);
                ui.label(RichText::new("No images yet").heading());
                ui.label(RichText::new(DROP_HINT).color(TEXT_WEAK));
                ui.add_space(10.0);
                actions(ui);
            });
            recents(ui, state);
        });
}

#[cfg(not(target_arch = "wasm32"))]
fn actions(ui: &mut Ui) {
    use super::widgets::buttons::secondary_button;
    let width = 216.0;
    ui.allocate_ui_with_layout(
        egui::vec2(width, 24.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            if primary_button(ui, "Add images...").clicked() {
                Command::AddImages.schedule();
            }
            if secondary_button(ui, "Open config...").clicked() {
                Command::OpenConfig.schedule();
            }
        },
    );
}

#[cfg(target_arch = "wasm32")]
fn actions(ui: &mut Ui) {
    if primary_button(ui, "Add images...").clicked() {
        Command::AddImages.schedule();
    }
}

#[cfg(not(target_arch = "wasm32"))]
const CARD_SIZE: egui::Vec2 = egui::vec2(148.0, 84.0);
#[cfg(not(target_arch = "wasm32"))]
const CARD_GAP: f32 = 12.0;

#[cfg(not(target_arch = "wasm32"))]
fn cards_per_row(ui: &Ui) -> usize {
    (((ui.available_width() + CARD_GAP) / (CARD_SIZE.x + CARD_GAP)) as usize).clamp(1, 4)
}

#[cfg(not(target_arch = "wasm32"))]
fn recent_rows(ui: &Ui, state: &AppState) -> usize {
    state.ui.settings.recents.len().div_ceil(cards_per_row(ui))
}

#[cfg(target_arch = "wasm32")]
fn recent_rows(_ui: &Ui, _state: &AppState) -> usize {
    0
}

#[cfg(not(target_arch = "wasm32"))]
fn recents(ui: &mut Ui, state: &AppState) {
    use super::widgets::section::section_header;
    let paths = &state.ui.settings.recents;
    if paths.is_empty() {
        return;
    }
    ui.add_space(18.0);
    ui.vertical_centered(|ui| section_header(ui, "Recent"));
    ui.add_space(4.0);
    let columns = cards_per_row(ui);
    for (row, chunk) in paths.chunks(columns).enumerate() {
        let width = chunk.len() as f32 * (CARD_SIZE.x + CARD_GAP) - CARD_GAP;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = CARD_GAP;
            ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
            for (column, path) in chunk.iter().enumerate() {
                recent_card(ui, row * columns + column, path);
            }
        });
        ui.add_space(CARD_GAP);
    }
}

#[cfg(target_arch = "wasm32")]
fn recents(_ui: &mut Ui, _state: &AppState) {}

#[cfg(not(target_arch = "wasm32"))]
fn recent_card(ui: &mut Ui, index: usize, path: &str) {
    use super::theme::colors::{CARD_BG, CARD_BG_HOVER, TEXT};
    use super::widgets::paint_line;
    use egui::{Align2, Role, Sense, WidgetInfo};

    let (rect, response) = ui.allocate_exact_size(CARD_SIZE, Sense::click());
    let response = response.on_hover_text(path);
    let name = std::path::Path::new(path).file_name().map_or_else(
        || path.to_owned(),
        |name| name.to_string_lossy().to_string(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, &name));
    let fill = if response.hovered() {
        CARD_BG_HOVER
    } else {
        CARD_BG
    };
    ui.painter().rect_filled(rect, 8.0, fill);
    let dir = std::path::Path::new(path)
        .parent()
        .map(|dir| dir.display().to_string())
        .unwrap_or_default();
    let body = rect.shrink(8.0);
    let (top, bottom) = body.split_top_bottom_at_y(body.top() + 28.0);
    let (name_rect, dir_rect) = bottom.split_top_bottom_at_y(bottom.top() + 20.0);
    paint_line(
        ui,
        top,
        Align2::CENTER_CENTER,
        RichText::new(char::from(Icon::FileCog)).size(20.0),
        TEXT_WEAK,
    );
    paint_line(
        ui,
        name_rect,
        Align2::CENTER_CENTER,
        RichText::new(&name),
        TEXT,
    );
    paint_line(
        ui,
        dir_rect,
        Align2::CENTER_CENTER,
        RichText::new(dir).small(),
        HINT_TEXT,
    );
    if response.clicked() {
        Command::OpenRecent(index).schedule();
    }
}
