//! All colors of the app. Surfaces get lighter with elevation, so borders can stay subtle.
use egui::Color32;

/// Text fields, JSON panel background.
pub const EXTREME_BG: Color32 = Color32::from_rgb(0x12, 0x12, 0x14);
/// Preview area.
pub const PANEL_BG: Color32 = Color32::from_rgb(0x1e, 0x1e, 0x20);
/// Sprites panel, inspector.
pub const SIDEBAR_BG: Color32 = Color32::from_rgb(0x26, 0x26, 0x28);
/// Sheets, popups, striping.
pub const WINDOW_BG: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2c);
/// Menu, toolbar, status bar.
pub const TOOLBAR_BG: Color32 = Color32::from_rgb(0x2b, 0x2b, 0x2d);
/// Button faces, grouped boxes.
pub const CARD_BG: Color32 = Color32::from_rgb(0x30, 0x30, 0x33);
pub const CARD_BG_HOVER: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3d);
/// Hairlines between bars.
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x35, 0x35, 0x38);
/// Controls, windows.
pub const BORDER: Color32 = Color32::from_rgb(0x3d, 0x3d, 0x41);
/// Slider rail, checkbox well.
pub const CONTROL_TRACK: Color32 = Color32::from_rgb(0x4a, 0x4a, 0x4e);

pub const TEXT: Color32 = Color32::from_rgb(236, 236, 240);
pub const TEXT_WEAK: Color32 = Color32::from_rgb(168, 168, 174);
/// Placeholder and disabled text.
pub const HINT_TEXT: Color32 = Color32::from_rgb(146, 146, 152);
/// Uppercase section titles.
pub const SECTION_LABEL: Color32 = Color32::from_rgb(0x8e, 0x8e, 0x93);

/// Focus rings, active toggles, slider fill, sprite outlines. White on it is only about
/// 3.4:1, so text never sits directly on this color; use `ACCENT_FILL` behind text.
pub const ACCENT: Color32 = Color32::from_rgb(0xe6, 0x66, 0x01);
/// Primary button behind white text (about 5:1).
pub const ACCENT_FILL: Color32 = Color32::from_rgb(0xbc, 0x4e, 0x00);
/// Selected rows and toggles.
pub const SELECTION_BG: Color32 = Color32::from_rgb(0x7a, 0x3a, 0x08);

/// Error text and strokes.
pub const DELETE_RED_SOFT: Color32 = Color32::from_rgb(0xff, 0x6b, 0x61);
pub const WARNING: Color32 = Color32::from_rgb(0xff, 0xcc, 0x33);

/// Checkerboard behind the atlas.
pub const CHECKER_BASE: Color32 = Color32::from_gray(48);
pub const CHECKER_ALT: Color32 = Color32::from_gray(64);
/// Under the atlas and over it while a new one is building.
pub const CANVAS_SHADOW: Color32 = Color32::from_black_alpha(120);
/// Behind the message of the drop overlay.
pub const OVERLAY_DIM: Color32 = Color32::from_black_alpha(160);
pub const WINDOW_SHADOW: Color32 = Color32::from_black_alpha(140);
pub const POPUP_SHADOW: Color32 = Color32::from_black_alpha(110);

/// JSON syntax colors.
pub const JSON_KEY: Color32 = Color32::from_rgb(125, 190, 255);
pub const JSON_STRING: Color32 = Color32::from_rgb(160, 215, 145);
pub const JSON_NUMBER: Color32 = Color32::from_rgb(245, 195, 120);
pub const JSON_BOOLEAN: Color32 = Color32::from_rgb(210, 165, 245);
pub const JSON_NULL: Color32 = Color32::from_rgb(190, 190, 190);
