//! Shared styled widgets.
pub mod buttons;
pub mod section;

use egui::{Align2, Color32, Rect, RichText, TextStyle, TextWrapMode, Ui, WidgetText};

/// Paints one truncated line, for a widget that already gives its own accessible name.
/// A `Label` would announce the same text a second time.
pub fn paint_line(ui: &Ui, rect: Rect, align: Align2, text: RichText, color: Color32) {
    let galley = WidgetText::from(text).into_galley(
        ui,
        Some(TextWrapMode::Truncate),
        rect.width(),
        TextStyle::Body,
    );
    let pos = align.align_size_within_rect(galley.size(), rect).min;
    ui.painter().galley(pos, galley, color);
}
