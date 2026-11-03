//! Font setup: SF Pro on macOS, egui's built-ins elsewhere, and the Lucide icon font as
//! the last Proportional fallback so icon codepoints render in any `RichText`.
//!
//! The icon font is a subset (`cargo xtask icons-subset`), so only the icons in use ship.
use egui::{Context, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// Lucide Font IDs are stable, so the subset keeps every codepoint stable.
const ICON_FONT: &[u8] = include_bytes!("../../../assets/fonts/lucide-subset.ttf");

/// Candidate system UI font files, most preferred first.
#[cfg(target_os = "macos")]
const SYSTEM_UI_FONT_PATHS: &[&str] = &[
    "/System/Library/Fonts/SFNS.ttf",
    "/System/Library/Fonts/SFNSText.ttf",
    "/System/Library/Fonts/HelveticaNeue.ttc",
];

/// Production font setup, preferring the macOS system UI font.
pub fn register_app_fonts(ctx: &Context) {
    ctx.set_fonts(font_definitions(load_system_ui_font()));
}

/// Machine-independent font setup for snapshot tests.
#[cfg(test)]
pub fn register_test_fonts(ctx: &Context) {
    ctx.set_fonts(font_definitions(None));
}

#[cfg(target_os = "macos")]
fn load_system_ui_font() -> Option<Vec<u8>> {
    for path in SYSTEM_UI_FONT_PATHS {
        match std::fs::read(path) {
            Ok(bytes) => {
                log::info!("Using system UI font {path}");
                return Some(bytes);
            }
            Err(error) => log::debug!("System font {path} unavailable: {error}"),
        }
    }
    log::warn!("No macOS system UI font found; using egui's default font");
    None
}

#[cfg(not(target_os = "macos"))]
fn load_system_ui_font() -> Option<Vec<u8>> {
    None
}

fn font_definitions(system_font: Option<Vec<u8>>) -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    if let Some(bytes) = system_font {
        fonts
            .font_data
            .insert("system".to_owned(), FontData::from_owned(bytes).into());
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "system".to_owned());
    }
    // Lucide ships no hinting instructions, so hinting only rounds its phantom points,
    // which jitters icon bounds by a pixel.
    let mut icon_font = FontData::from_static(ICON_FONT);
    icon_font.tweak.hinting = Some(false);
    fonts
        .font_data
        .insert("lucide".to_owned(), Arc::new(icon_font));
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .push("lucide".to_owned());
    fonts
}
