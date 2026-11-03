//! Panels and widgets. They render from `AppState` and schedule `Command`s.
mod about;
mod inspector;
pub mod json_panel;
pub mod json_preview;
mod menu_bar;
pub mod preview;
mod sprites_panel;
mod start_screen;
mod status_bar;
pub mod theme;
pub mod toasts;
mod toolbar;
mod widgets;

use crate::state::AppState;
use egui::{Context, FontFamily, FontId, Frame, Margin, Stroke};
use lucide_icons::Icon;
use theme::colors::{ACCENT, BORDER_SUBTLE, OVERLAY_DIM, SIDEBAR_BG, TEXT, TOOLBAR_BG};

#[cfg(not(target_arch = "wasm32"))]
const DROP_MESSAGE: &str = "Drop images, a folder or a config here";
#[cfg(target_arch = "wasm32")]
const DROP_MESSAGE: &str = "Drop images here";

/// Fonts and style that have to be set up once.
pub fn install(ctx: &Context, native: bool) {
    if native {
        theme::fonts::register_app_fonts(ctx);
    } else {
        #[cfg(test)]
        theme::fonts::register_test_fonts(ctx);
    }
    theme::configure_theme(ctx);
}

pub fn icon_label(icon: Icon, text: &str) -> String {
    format!("{}  {}", char::from(icon), text)
}

fn bar(margin: Margin) -> Frame {
    Frame::new().fill(TOOLBAR_BG).inner_margin(margin)
}

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    let ctx = ui.ctx().clone();
    if !state.ui.native_menu {
        egui::Panel::top("menu_bar")
            .frame(bar(Margin::symmetric(8, 2)))
            .show(ui, |ui| menu_bar::show(ui, state));
    }
    egui::Panel::top("toolbar")
        .frame(bar(Margin::ZERO).stroke(Stroke::new(1.0, BORDER_SUBTLE)))
        .exact_size(toolbar::TOOLBAR_HEIGHT)
        .show(ui, |ui| toolbar::show(ui, state));
    let status_top = egui::Panel::bottom("status_bar")
        .frame(bar(Margin::symmetric(10, 0)))
        .exact_size(status_bar::STATUS_BAR_HEIGHT)
        .show(ui, |ui| status_bar::show(ui, state))
        .response
        .rect
        .top();

    let mut show_sprites = state.ui.settings.show_sprites;
    egui::Panel::left("sprites")
        .resizable(true)
        .default_size(240.0)
        .size_range(200.0..=360.0)
        .frame(sidebar())
        .show_collapsible(ui, &mut show_sprites, |ui| sprites_panel::show(ui, state));
    let mut show_inspector = state.ui.inspector_visible();
    egui::Panel::right("inspector")
        .resizable(true)
        .default_size(280.0)
        .size_range(260.0..=380.0)
        .frame(sidebar())
        .show_collapsible(ui, &mut show_inspector, |ui| inspector::show(ui, state));

    egui::CentralPanel::default()
        .frame(Frame::new().fill(theme::colors::PANEL_BG))
        .show(ui, |ui| {
            json_panel::show(ui, state);
            preview::show_panel(ui, state);
        });

    about::show(&ctx, state);
    drop_overlay(&ctx);
    state.ui.toasts.show(&ctx, status_top);
}

fn sidebar() -> Frame {
    Frame::new()
        .fill(SIDEBAR_BG)
        .inner_margin(Margin::symmetric(12, 10))
}

fn drop_overlay(ctx: &Context) {
    if ctx.input(|i| i.raw.hovered_files.is_empty()) {
        return;
    }
    let rect = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::unique("drop_overlay"),
    ));
    painter.rect_filled(rect, 0.0, OVERLAY_DIM);
    painter.rect_stroke(
        rect.shrink(12.0),
        12.0,
        Stroke::new(2.0, ACCENT),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        DROP_MESSAGE,
        FontId::new(20.0, FontFamily::Proportional),
        TEXT,
    );
}
