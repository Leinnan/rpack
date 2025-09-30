use std::{ffi::OsString, path::PathBuf, slice::Iter, str::FromStr};

#[cfg(not(target_arch = "wasm32"))]
pub fn load_fonts(ctx: &egui::Context) {
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
}

#[cfg(not(target_arch = "wasm32"))]
fn get_fonts() -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    get_fonts_from_list(
        &["SFNSRounded", "aptos"],
        &["SFNSRounded", "aptos-semibold"],
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn get_fonts_from_list(
    regular: &[&'static str],
    semibold: &[&'static str],
) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let Some(regular) = regular
        .iter()
        .find_map(|f| try_get_font_path_from_os(f).and_then(|path| std::fs::read(path).ok()))
    else {
        anyhow::bail!("Failed to find a suitable font");
    };
    let Some(semibold) = semibold
        .iter()
        .find_map(|f| try_get_font_path_from_os(f).and_then(|path| std::fs::read(path).ok()))
    else {
        anyhow::bail!("Failed to find a suitable font");
    };

    Ok((regular, semibold))
}

#[allow(unused)]
trait FontLoader {
    fn check_for_font(self, font_file_names: &[PathBuf]) -> Option<PathBuf>;
}

impl FontLoader for &PathBuf {
    fn check_for_font(self, font_file_names: &[PathBuf]) -> Option<PathBuf> {
        font_file_names.iter().find_map(|f| {
            if self.join(f).exists() {
                Some(self.join(f))
            } else {
                None
            }
        })
    }
}
impl FontLoader for &str {
    fn check_for_font(self, font_file_names: &[PathBuf]) -> Option<PathBuf> {
        let path = PathBuf::from_str(self).ok()?;
        font_file_names.iter().find_map(|f| {
            if path.join(f).exists() {
                Some(path.join(f))
            } else {
                None
            }
        })
    }
}
impl FontLoader for (&str, &str) {
    fn check_for_font(self, font_file_names: &[PathBuf]) -> Option<PathBuf> {
        let path = PathBuf::from_str(self.0).ok()?;
        let path = path.join(self.1);
        path.check_for_font(font_file_names)
    }
}
impl FontLoader for Iter<'_, &'static str> {
    fn check_for_font(self, font_file_names: &[PathBuf]) -> Option<PathBuf> {
        for path in self.into_iter().flat_map(|s| PathBuf::from_str(s).ok()) {
            if let Some(font_path) = path.check_for_font(font_file_names) {
                return Some(font_path);
            }
        }
        None
    }
}

impl FontLoader for (Option<OsString>, &str) {
    fn check_for_font(self, font_file_names: &[PathBuf]) -> Option<PathBuf> {
        let path = self
            .0
            .and_then(|f| PathBuf::from_str(&f.to_string_lossy()).ok())?;
        let path = path.join(self.1);
        path.check_for_font(font_file_names)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn try_get_font_path_from_os(font_name: &str) -> Option<PathBuf> {
    let file_paths_to_check = if font_name.ends_with(".ttf") || font_name.ends_with(".otf") {
        vec![PathBuf::from(font_name)]
    } else {
        vec![
            PathBuf::from(format!("{}.ttf", font_name)),
            PathBuf::from(format!("{}.otf", font_name)),
        ]
    };
    get_font_paths()
        .iter()
        .find_map(|dir| dir.check_for_font(&file_paths_to_check))
}

#[cfg(not(target_arch = "wasm32"))]
fn build_path_from_env(env_var: &str, subpath: &str) -> Option<PathBuf> {
    std::env::var(env_var)
        .ok()
        .and_then(|d| PathBuf::from_str(d.as_str()).map(|p| p.join(subpath)).ok())
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(unused)]
pub fn get_system_fonts() -> Vec<String> {
    get_font_paths()
        .iter()
        .flat_map(|s| {
            let dir = std::fs::read_dir(s).ok()?;
            let dir_entries: Vec<String> = dir
                .flat_map(|e| {
                    let entry = e.ok()?;
                    if entry.file_name().to_string_lossy().ends_with("ttf")
                        || entry.file_name().to_string_lossy().ends_with("otf")
                    {
                        Some(
                            entry
                                .file_name()
                                .to_string_lossy()
                                .replace(".ttf", "")
                                .replace(".otf", ""),
                        )
                    } else {
                        None
                    }
                })
                .collect();
            Some(dir_entries)
        })
        .flatten()
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_font_paths() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    if let Some(path) = build_path_from_env("LOCALAPPDATA", "Microsoft/Windows/Fonts") {
        vec![path]
    } else {
        vec![]
    }
    #[cfg(target_os = "linux")]
    {
        let mut results = vec![
            PathBuf::from_str("/usr/share/fonts").unwrap_or_default(),
            PathBuf::from_str("/usr/share/fonts/truetype").unwrap_or_default(),
        ];
        if let Some(path) = build_path_from_env("HOME", ".local/share/fonts") {
            results.push(path);
        }
        results
    }
    #[cfg(target_os = "macos")]
    {
        let mut results = vec![PathBuf::from_str("/System/Library/Fonts").unwrap_or_default()];
        if let Some(path) = build_path_from_env("HOME", "Library/Fonts") {
            results.push(path);
        }
        results
    }
}
