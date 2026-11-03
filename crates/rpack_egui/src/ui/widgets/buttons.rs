//! Toolbar and sheet buttons.
use crate::commands::Command;
use crate::menu;
use crate::ui::theme::colors::{
    ACCENT_FILL, BORDER, CARD_BG, CARD_BG_HOVER, CONTROL_TRACK, HINT_TEXT, TEXT, TEXT_WEAK,
};
use egui::{
    Align2, Button, Color32, CornerRadius, FontId, Response, RichText, Role, Sense, Stroke, Ui,
    Vec2, WidgetInfo,
};
use lucide_icons::Icon;

const ROUNDING: u8 = 6;

/// Filled accent button for the default action of a screen.
pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    ui.add(
        Button::new(RichText::new(text).color(TEXT).strong())
            .fill(ACCENT_FILL)
            .stroke(Stroke::new(1.0, ACCENT_FILL))
            .min_size(Vec2::new(72.0, 24.0))
            .corner_radius(CornerRadius::same(ROUNDING)),
    )
}

/// Outlined low-emphasis button.
pub fn secondary_button(ui: &mut Ui, text: &str) -> Response {
    ui.add(
        Button::new(RichText::new(text).color(TEXT))
            .fill(CARD_BG)
            .stroke(Stroke::new(1.0, BORDER))
            .min_size(Vec2::new(72.0, 24.0))
            .corner_radius(CornerRadius::same(ROUNDING)),
    )
}

/// What an icon button shows on hover: its name, plus the shortcut of its command.
/// The name alone is the accessible label.
pub struct Tip<'a> {
    name: &'a str,
    command: Option<Command>,
}

impl<'a> From<&'a str> for Tip<'a> {
    fn from(name: &'a str) -> Self {
        Self {
            name,
            command: None,
        }
    }
}

impl<'a> Tip<'a> {
    pub fn command(name: &'a str, command: Command) -> Self {
        Self {
            name,
            command: Some(command),
        }
    }

    fn text(&self, ctx: &egui::Context) -> String {
        match self.command.and_then(menu::shortcut_for) {
            Some(shortcut) => format!("{}  {}", self.name, ctx.format_shortcut(shortcut)),
            None => self.name.to_owned(),
        }
    }
}

/// The fill and content color of an icon button.
fn colors(active: bool, response: &Response, enabled: bool) -> (Option<Color32>, Color32) {
    if !enabled {
        return (None, HINT_TEXT);
    }
    let (hovered, pressed) = (response.hovered(), response.is_pointer_button_down_on());
    let fill = match (active, hovered || pressed) {
        (_, _) if pressed => Some(CONTROL_TRACK),
        (true, true) => Some(CONTROL_TRACK),
        (true, false) | (false, true) => Some(CARD_BG_HOVER),
        (false, false) => None,
    };
    let color = if active || hovered { TEXT } else { TEXT_WEAK };
    (fill, color)
}

/// Unlike egui's `Button`, whose frame (and so its content) shifts when hover adds a
/// stroke, nothing here depends on the state but the colors.
fn icon_button<'a>(
    ui: &mut Ui,
    icon: Icon,
    size: Vec2,
    glyph_size: f32,
    active: bool,
    enabled: bool,
    tip: Tip<'a>,
) -> Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    if ui.is_rect_visible(rect) {
        let (fill, color) = colors(active, &response, enabled);
        if let Some(fill) = fill {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(ROUNDING), fill);
        }
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            char::from(icon),
            FontId::proportional(glyph_size),
            color,
        );
    }
    let response = response.on_hover_ui(|ui| {
        ui.label(tip.text(ui.ctx()));
    });
    response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, tip.name));
    response
}

/// Borderless toolbar icon button, filled while `active` (an option that is on).
pub fn toolbar_toggle<'a>(
    ui: &mut Ui,
    icon: Icon,
    active: bool,
    tip: impl Into<Tip<'a>>,
) -> Response {
    let size = Vec2::new(28.0, 24.0);
    icon_button(ui, icon, size, 16.0, active, true, tip.into())
}

/// Toolbar icon button for an action that may be unavailable.
pub fn toolbar_icon<'a>(
    ui: &mut Ui,
    icon: Icon,
    enabled: bool,
    tip: impl Into<Tip<'a>>,
) -> Response {
    let size = Vec2::new(28.0, 24.0);
    icon_button(ui, icon, size, 16.0, false, enabled, tip.into())
}

/// Small borderless icon button for inline actions.
pub fn small_icon_button<'a>(
    ui: &mut Ui,
    icon: Icon,
    enabled: bool,
    tip: impl Into<Tip<'a>>,
) -> Response {
    icon_button(
        ui,
        icon,
        Vec2::splat(22.0),
        14.0,
        false,
        enabled,
        tip.into(),
    )
}
