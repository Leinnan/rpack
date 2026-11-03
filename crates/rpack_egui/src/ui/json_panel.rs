//! The resizable panel showing the atlas JSON, below the preview.
use super::theme::colors::{BORDER_SUBTLE, EXTREME_BG};
use super::widgets::{buttons::small_icon_button, section::section_header};
use crate::commands::Command;
use crate::state::AppState;
use egui::{Align, Frame, Layout, Margin, Stroke, Ui};
use lucide_icons::Icon;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let AppState {
        build,
        ui: ui_state,
        ..
    } = state;
    if !ui_state.settings.display_json {
        return;
    }
    let Some(built) = &build.output else {
        return;
    };
    egui::Panel::bottom("json_panel")
        .resizable(true)
        .default_size(200.0)
        .size_range(80.0..=600.0)
        .frame(
            Frame::new()
                .fill(EXTREME_BG)
                .stroke(Stroke::new(1.0, BORDER_SUBTLE))
                .inner_margin(Margin::symmetric(12, 6)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                section_header(ui, "Atlas JSON");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if small_icon_button(ui, Icon::Copy, true, "Copy JSON").clicked() {
                        Command::CopyJson.schedule();
                    }
                });
            });
            egui::ScrollArea::vertical()
                .id_salt("json_vertical_scroll")
                .auto_shrink(false)
                .show(ui, |ui| {
                    ui_state.json_preview.show(ui, &built.atlas_asset_json);
                });
        });
}
