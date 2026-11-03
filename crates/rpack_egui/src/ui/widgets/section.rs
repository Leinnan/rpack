//! Inspector section headers.
use crate::ui::theme::colors::{CARD_BG_HOVER, SECTION_LABEL, TEXT_WEAK};
use egui::{Align, CornerRadius, Layout, RichText, Sense, Ui, UiBuilder, Vec2};
use lucide_icons::Icon;

const HEADER_HEIGHT: f32 = 24.0;
const SECTION_BOTTOM: f32 = 8.0;

/// Small uppercase group title.
pub fn section_header(ui: &mut Ui, title: &str) {
    ui.label(
        RichText::new(title.to_uppercase())
            .size(10.5)
            .strong()
            .color(SECTION_LABEL),
    );
}

/// Collapsible section: an uppercase title with a disclosure chevron. The open state
/// is kept in egui memory under `title`.
pub fn inspector_section(ui: &mut Ui, title: &str, body: impl FnOnce(&mut Ui)) {
    let id = ui.scope_id().with(("inspector_section", title));
    let mut open = ui.data_mut(|d| *d.get_persisted_mut_or(id, true));
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), HEADER_HEIGHT),
        Sense::click(),
    );
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), CARD_BG_HOVER);
    }
    let mut header = ui.new_child(
        UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(4.0, 0.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    section_header(&mut header, title);
    header.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let chevron = if open {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        };
        ui.label(
            RichText::new(char::from(chevron))
                .size(12.0)
                .color(TEXT_WEAK),
        );
    });
    if response.clicked() {
        open = !open;
        ui.data_mut(|d| d.insert_persisted(id, open));
    }
    if open {
        ui.add_space(2.0);
        body(ui);
    }
    ui.add_space(SECTION_BOTTOM);
}
