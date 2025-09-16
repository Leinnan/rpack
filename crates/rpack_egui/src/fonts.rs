pub fn setup_custom_fonts(ctx: &egui::Context) {
    // Start with the default fonts (we will be adding to them rather than replacing them).
    let mut fonts = egui::FontDefinitions::default();
    if let Ok((regular, semibold)) = get_fonts() {
        fonts.font_data.insert(
            "regular".to_owned(),
            egui::FontData::from_owned(regular).into(),
        );
        fonts.font_data.insert(
            "semibold".to_owned(),
            egui::FontData::from_owned(semibold).into(),
        );

        // Put my font first (highest priority) for proportional text:
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "regular".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Name("semibold".into()))
            .or_default()
            .insert(0, "semibold".to_owned());

        // Put my font as last fallback for monospace:
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("regular".to_owned());

        // Tell egui to use these fonts:
        ctx.set_fonts(fonts);
    }

    ctx.all_styles_mut(|style| {
        for font_id in style.text_styles.values_mut() {
            font_id.size *= 1.4;
        }
    });
}

#[cfg(not(windows))]
fn get_fonts() -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    use std::fs;

    let font_path = std::path::Path::new("/System/Library/Fonts");

    let regular = fs::read(font_path.join("SFNSRounded.ttf"))?;
    let semibold = fs::read(font_path.join("SFCompact.ttf"))?;

    Ok((regular, semibold))
}

#[cfg(windows)]
fn get_fonts() -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    use std::fs;
    let app_data = std::env::var("APPDATA")?;
    let font_path = std::path::Path::new(&app_data);

    let regular = fs::read(font_path.join("../Local/Microsoft/Windows/Fonts/aptos.ttf"))?;
    let semibold = fs::read(font_path.join("../Local/Microsoft/Windows/Fonts/aptos-semibold.ttf"))?;

    Ok((regular, semibold))
}
