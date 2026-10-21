use egui::{Color32, FontId, TextFormat, Ui, text::LayoutJob};
use serde_json::Value;

#[derive(Clone, PartialEq)]
struct Palette {
    font: FontId,
    colors: [Color32; 6],
}

impl Palette {
    fn from_ui(ui: &Ui) -> Self {
        let colors = if ui.visuals().dark_mode {
            [
                ui.visuals().text_color(),
                Color32::from_rgb(125, 190, 255),
                Color32::from_rgb(160, 215, 145),
                Color32::from_rgb(245, 195, 120),
                Color32::from_rgb(210, 165, 245),
                Color32::from_rgb(190, 190, 190),
            ]
        } else {
            [
                ui.visuals().text_color(),
                Color32::from_rgb(0, 80, 160),
                Color32::from_rgb(30, 110, 40),
                Color32::from_rgb(145, 75, 0),
                Color32::from_rgb(120, 45, 160),
                Color32::from_rgb(90, 90, 90),
            ]
        };
        Self {
            font: egui::TextStyle::Monospace.resolve(ui.style()),
            colors,
        }
    }
}

#[derive(Clone, Copy)]
enum Token {
    Punctuation,
    Key,
    String,
    Number,
    Boolean,
    Null,
}

#[derive(Default)]
pub(crate) struct JsonPreview {
    cached: Option<(Palette, LayoutJob)>,
}

impl JsonPreview {
    // Call whenever the source Value changes. Hidden previews are formatted lazily.
    pub(crate) fn invalidate(&mut self) {
        self.cached = None;
    }

    fn layout(&mut self, value: &Value, palette: Palette) -> &LayoutJob {
        if self.cached.as_ref().is_none_or(|(old, _)| *old != palette) {
            let mut job = LayoutJob::default();
            format_value(&mut job, value, &palette, 0);
            self.cached = Some((palette, job));
        }
        &self.cached.as_ref().expect("preview initialized").1
    }

    pub(crate) fn show(&mut self, ui: &mut Ui, value: &Value) {
        let palette = Palette::from_ui(ui);
        let job = self.layout(value, palette);
        egui::ScrollArea::horizontal()
            .id_salt("json_preview_scroll")
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.add(
                        egui::Label::new(job.clone())
                            .selectable(true)
                            .wrap_mode(egui::TextWrapMode::Extend),
                    );
                });
            });
    }
}

fn append(job: &mut LayoutJob, text: &str, token: Token, palette: &Palette) {
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: palette.font.clone(),
            color: palette.colors[token as usize],
            ..Default::default()
        },
    );
}

fn indent(job: &mut LayoutJob, depth: usize, palette: &Palette) {
    append(job, &"  ".repeat(depth), Token::Punctuation, palette);
}

fn format_value(job: &mut LayoutJob, value: &Value, palette: &Palette, depth: usize) {
    match value {
        Value::Object(entries) => {
            append(job, "{", Token::Punctuation, palette);
            for (index, (key, value)) in entries.iter().enumerate() {
                append(
                    job,
                    if index == 0 { "\n" } else { ",\n" },
                    Token::Punctuation,
                    palette,
                );
                indent(job, depth + 1, palette);
                append(
                    job,
                    &Value::String(key.clone()).to_string(),
                    Token::Key,
                    palette,
                );
                append(job, ": ", Token::Punctuation, palette);
                format_value(job, value, palette, depth + 1);
            }
            if !entries.is_empty() {
                append(job, "\n", Token::Punctuation, palette);
                indent(job, depth, palette);
            }
            append(job, "}", Token::Punctuation, palette);
        }
        Value::Array(entries) => {
            append(job, "[", Token::Punctuation, palette);
            for (index, value) in entries.iter().enumerate() {
                append(
                    job,
                    if index == 0 { "\n" } else { ",\n" },
                    Token::Punctuation,
                    palette,
                );
                indent(job, depth + 1, palette);
                format_value(job, value, palette, depth + 1);
            }
            if !entries.is_empty() {
                append(job, "\n", Token::Punctuation, palette);
                indent(job, depth, palette);
            }
            append(job, "]", Token::Punctuation, palette);
        }
        scalar => {
            let token = match scalar {
                Value::String(_) => Token::String,
                Value::Number(_) => Token::Number,
                Value::Bool(_) => Token::Boolean,
                _ => Token::Null,
            };
            append(job, &scalar.to_string(), token, palette);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn palette() -> Palette {
        Palette {
            font: FontId::monospace(14.0),
            colors: std::array::from_fn(|i| Color32::from_gray((i * 30) as u8)),
        }
    }

    #[test]
    fn formatting_preserves_json_and_utf8_ranges() {
        for value in [
            json!({"é\"": [null, true, false, -123, 1.25, "雪\n\t\\\"", {}, []]}),
            json!({}),
            json!([]),
            json!(null),
            json!("hello"),
            json!(1e20),
        ] {
            let mut preview = JsonPreview::default();
            let job = preview.layout(&value, palette());
            assert_eq!(job.text, serde_json::to_string_pretty(&value).unwrap());
            assert_eq!(serde_json::from_str::<Value>(&job.text).unwrap(), value);
            for section in &job.sections {
                assert!(
                    job.text
                        .get(section.byte_range.start.0..section.byte_range.end.0)
                        .is_some()
                );
            }
        }
    }

    #[test]
    fn token_categories_have_distinct_colors() {
        let palette = palette();
        let mut preview = JsonPreview::default();
        let job = preview.layout(&json!({"key": ["value", 42, true, null]}), palette.clone());
        for (text, token) in [
            ("{", Token::Punctuation),
            ("\"key\"", Token::Key),
            ("\"value\"", Token::String),
            ("42", Token::Number),
            ("true", Token::Boolean),
            ("null", Token::Null),
        ] {
            let start = job.text.find(text).unwrap();
            let section = job
                .sections
                .iter()
                .find(|s| s.byte_range.start.0 <= start && s.byte_range.end.0 >= start + text.len())
                .unwrap();
            assert_eq!(section.format.color, palette.colors[token as usize]);
        }
    }

    #[test]
    fn preview_renders_large_values_in_both_themes() {
        let ctx = egui::Context::default();
        let mut preview = JsonPreview::default();
        let value = json!({"frames": (0..500).map(|i| json!({
            "name": format!("sprite_{i}_{}", "long_name".repeat(20)),
            "size": [16, 32],
        })).collect::<Vec<_>>()});
        for visuals in [egui::Visuals::dark(), egui::Visuals::light()] {
            ctx.set_visuals(visuals);
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| preview.show(ui, &value));
            });
            // This headless test has no renderer to consume texture uploads.
            output.textures_delta.clear();
            assert!(!output.shapes.is_empty());
            assert_eq!(
                preview.cached.as_ref().unwrap().1.text,
                serde_json::to_string_pretty(&value).unwrap()
            );
        }
    }

    #[test]
    fn invalidation_and_style_changes_refresh_cache() {
        let mut preview = JsonPreview::default();
        preview.layout(&json!("old"), palette());
        preview.invalidate();
        assert!(preview.cached.is_none());
        assert_eq!(preview.layout(&json!("new"), palette()).text, "\"new\"");
        let mut changed = palette();
        changed.colors[Token::String as usize] = Color32::RED;
        changed.font.size = 20.0;
        let job = preview.layout(&json!("new"), changed.clone());
        assert_eq!(job.sections[0].format.color, Color32::RED);
        assert_eq!(job.sections[0].format.font_id, changed.font);
    }
}
