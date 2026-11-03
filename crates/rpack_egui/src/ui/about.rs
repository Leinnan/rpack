//! The About sheet.
use super::theme::colors::{ACCENT, TEXT_WEAK};
use super::widgets::buttons::secondary_button;
use crate::state::{AppState, Dialog};
use egui::{Context, Id, Modal, RichText};

// Snapshot tests must not change with every commit.
const GIT_HASH: &str = if cfg!(test) { "test" } else { env!("GIT_HASH") };

pub fn show(ctx: &Context, state: &mut AppState) {
    if state.ui.dialog != Some(Dialog::About) {
        return;
    }
    let mut close = false;
    let response = Modal::new(Id::unique("about")).show(ctx, |ui| {
        ui.set_width(280.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("rPack").size(22.0).strong().color(ACCENT));
            ui.label(
                RichText::new(format!(
                    "Version {} ({GIT_HASH})",
                    env!("CARGO_PKG_VERSION")
                ))
                .color(TEXT_WEAK),
            );
            ui.add_space(8.0);
            ui.label("Sprite atlas packer");
            ui.hyperlink_to("Made by Mev Lyshkin", "https://www.mevlyshkin.com/");
            ui.hyperlink_to("Homepage", env!("CARGO_PKG_HOMEPAGE"));
            ui.add_space(12.0);
            close = secondary_button(ui, "Close").clicked();
        });
    });
    if close || response.should_close() {
        state.ui.dialog = None;
    }
}
