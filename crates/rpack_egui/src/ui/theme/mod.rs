//! Visuals and spacing of the dark look.
pub mod colors;
pub mod fonts;

use colors::{
    ACCENT, BORDER, BORDER_SUBTLE, CARD_BG, CARD_BG_HOVER, CONTROL_TRACK, DELETE_RED_SOFT,
    EXTREME_BG, HINT_TEXT, PANEL_BG, POPUP_SHADOW, SELECTION_BG, TEXT, WARNING, WINDOW_BG,
    WINDOW_SHADOW,
};
use egui::{Context, CornerRadius, FontFamily, FontId, TextStyle};

/// Install the theme on `ctx`.
pub fn configure_theme(ctx: &Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    let mut visuals = egui::Visuals::dark();
    // `override_text_color` stays unset so hint text keeps `weak_text_color`
    // and reads dimmer than typed text.
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.fg_stroke.color = TEXT;
    }
    visuals.weak_text_color = Some(HINT_TEXT);
    visuals.error_fg_color = DELETE_RED_SOFT;
    visuals.warn_fg_color = WARNING;
    visuals.panel_fill = PANEL_BG;
    visuals.window_fill = WINDOW_BG;
    visuals.faint_bg_color = WINDOW_BG;
    visuals.extreme_bg_color = EXTREME_BG;
    visuals.selection.bg_fill = SELECTION_BG;
    visuals.selection.stroke.color = ACCENT;
    visuals.hyperlink_color = ACCENT;
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.weak_bg_fill = ACCENT;
    visuals.widgets.active.bg_stroke.color = ACCENT;
    visuals.widgets.hovered.bg_fill = CARD_BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = CARD_BG_HOVER;
    visuals.widgets.hovered.bg_stroke.color = BORDER;
    visuals.widgets.open.weak_bg_fill = CARD_BG_HOVER;
    visuals.widgets.noninteractive.bg_fill = CARD_BG;
    visuals.widgets.noninteractive.bg_stroke.color = BORDER_SUBTLE;
    // `bg_fill` paints slider rails and checkbox wells, `weak_bg_fill`
    // button faces: rails stay lighter so they read against grouped boxes.
    visuals.widgets.inactive.bg_fill = CONTROL_TRACK;
    visuals.widgets.inactive.weak_bg_fill = CARD_BG;
    visuals.widgets.inactive.bg_stroke.color = BORDER;
    visuals.widgets.inactive.bg_stroke.width = 1.0;
    visuals.window_stroke.color = BORDER;
    visuals.window_stroke.width = 1.0;
    visuals.slider_trailing_fill = true;
    visuals.window_shadow = egui::Shadow {
        offset: [0, 12],
        blur: 40,
        spread: 0,
        color: WINDOW_SHADOW,
    };
    visuals.popup_shadow = egui::Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: POPUP_SHADOW,
    };
    ctx.set_visuals(visuals);

    let mut style = (*ctx.global_style()).clone();
    style.text_styles = [
        (
            TextStyle::Small,
            FontId::new(11.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(13.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Heading,
            FontId::new(15.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(12.0, FontFamily::Monospace),
        ),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(8.0, 3.0);
    style.spacing.interact_size.y = 22.0;
    style.spacing.indent = 14.0;
    style.spacing.window_margin = egui::Margin::same(12);
    style.spacing.menu_margin = egui::Margin::same(5);
    style.interaction.selectable_labels = false;
    // Controls get a 6 px radius; windows and sheets a softer one.
    style.visuals.widgets.active.corner_radius = CornerRadius::same(6);
    style.visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    style.visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    style.visuals.widgets.open.corner_radius = CornerRadius::same(6);
    style.visuals.widgets.noninteractive.corner_radius = CornerRadius::same(10);
    style.visuals.menu_corner_radius = CornerRadius::same(8);
    style.visuals.window_corner_radius = CornerRadius::same(12);
    ctx.set_global_style(style);
    ctx.options_mut(|options| options.zoom_with_keyboard = false);
}
