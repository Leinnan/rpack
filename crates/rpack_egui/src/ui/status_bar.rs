//! A hint on the left, atlas statistics on the right.
use super::theme::colors::TEXT_WEAK;
use crate::commands::Command;
use crate::state::AppState;
use egui::{Align, Button, Layout, RichText, Ui};

pub const STATUS_BAR_HEIGHT: f32 = 24.0;

pub fn show(ui: &mut Ui, state: &AppState) {
    profile_scope!("status_bar");
    ui.horizontal_centered(|ui| {
        let small = |text: &str| RichText::new(text).small().color(TEXT_WEAK);
        let hint = if state.build.building {
            "Building atlas..."
        } else if state.build.output.is_some() {
            "Scroll to zoom, drag to pan, double click to fit"
        } else {
            ""
        };
        ui.label(small(hint));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let Some(built) = &state.build.output else {
                return;
            };
            let zoom = format!("{}%", state.ui.preview.zoom_percent());
            if ui
                .add(Button::new(small(&zoom)).frame(false))
                .on_hover_text("Fit the atlas")
                .clicked()
            {
                Command::FitPreview.schedule();
            }
            let [width, height] = built.atlas_asset.size;
            let used: u64 = built
                .atlas_asset
                .frames
                .iter()
                .map(|f| u64::from(f.frame.w) * u64::from(f.frame.h))
                .sum();
            let total = (u64::from(width) * u64::from(height)).max(1);
            ui.label(small(&format!(
                "{} sprites  ·  {width}×{height}  ·  {:.0}% used",
                built.atlas_asset.frames.len(),
                used as f64 * 100.0 / total as f64
            )));
        });
    });
}
