use std::path::Path;

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

#[cfg(target_arch = "wasm32")]
fn get_fonts() -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let regular = include_bytes!("../static/JetBrainsMonoNL-Regular.ttf").to_vec();
    let semibold = include_bytes!("../static/JetBrainsMono-SemiBold.ttf").to_vec();

    Ok((regular, semibold))
}

#[cfg(not(target_arch = "wasm32"))]
fn get_fonts() -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let Some(regular) =
        try_get_font_from_list(&["JetBrainsMonoNL-Regular", "SFNSRounded", "aptos"])
    else {
        anyhow::bail!("Failed to find a suitable font");
    };
    let Some(semibold) =
        try_get_font_from_list(&["JetBrainsMono-SemiBold", "SFNSRounded", "aptos-semibold"])
    else {
        anyhow::bail!("Failed to find a suitable font");
    };

    Ok((regular, semibold))
}

#[cfg(not(target_arch = "wasm32"))]
fn try_get_font_from_list(font_names: &[&str]) -> Option<Vec<u8>> {
    for font_name in font_names {
        if let Some(font) = try_get_font(font_name) {
            return Some(font);
        }
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn try_get_font(font_name: &str) -> Option<Vec<u8>> {
    for dir in font_dirs() {
        if let Ok(font) = std::fs::read(Path::new(&dir).join(format!("{}.ttf", font_name))) {
            return Some(font);
        }
        if let Ok(font) = std::fs::read(Path::new(&dir).join(format!("{}.otf", font_name))) {
            return Some(font);
        }
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn font_dirs() -> Vec<String> {
    let mut dirs = Vec::new();

    #[cfg(target_os = "linux")]
    {
        dirs.push("/usr/share/fonts".into());
        dirs.push("/usr/share/fonts/truetype".into());
    }
    #[cfg(unix)]
    {
        use std::{path::PathBuf, str::FromStr};

        #[cfg(target_os = "macos")]
        {
            dirs.push("/System/Library/Fonts".into());
            if let Some(resources_font_dir) = std::env::current_exe().ok().and_then(|p| {
                p.ancestors()
                    .nth(2)
                    .map(|p| p.join("Resources/fonts").to_string_lossy().into_owned())
            }) {
                dirs.push(resources_font_dir);
            }
        }
        if let Some(home) =
            std::env::var_os("HOME").and_then(|s| PathBuf::from_str(&s.to_string_lossy()).ok())
        {
            #[cfg(target_os = "macos")]
            {
                dirs.push(format!("{}/Library/Fonts", home.display()));
            }
            #[cfg(target_os = "linux")]
            {
                dirs.push(format!("{}/.local/share/fonts", home.display()));
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(dir) = std::env::var("APPDATA") {
            let font_path = std::path::Path::new(&dir).join("../Local/Microsoft/Windows/Fonts/");
            dirs.push(font_path.display().to_string());
        }
    }

    dirs
}
