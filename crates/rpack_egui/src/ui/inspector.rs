//! Atlas settings and details of the selection.
use super::theme::colors::{HINT_TEXT, TEXT_WEAK};
use super::widgets::section::inspector_section;
use crate::state::{AppImageData, AppState};
use egui::containers::menu::MenuButton;
use egui::{Button, DragValue, Grid, Label, RichText, Slider, TextEdit, Ui, vec2};
use rpack_core::pipeline::ATLAS_SIZES;
use std::ops::RangeInclusive;

const AUTO_SIZE: &str = "Auto (2048, grows to fit)";

pub fn show(ui: &mut Ui, state: &mut AppState) {
    profile_scope!("inspector");
    egui::ScrollArea::vertical()
        .id_salt("inspector_scroll")
        .auto_shrink(false)
        .show(ui, |ui| {
            inspector_section(ui, "Atlas", |ui| atlas_settings(ui, state));
            inspector_section(ui, "Selection", |ui| selection(ui, state));
        });
}

fn atlas_settings(ui: &mut Ui, state: &mut AppState) {
    let settings = &mut state.project.settings;
    ui.spacing_mut().slider_width = 92.0;
    Grid::new("settings_grid")
        .num_columns(2)
        .spacing(vec2(8.0, 8.0))
        .show(ui, |ui| {
            let id = ui.label("File name").id;
            ui.add(TextEdit::singleline(&mut settings.output_path).desired_width(f32::INFINITY))
                .labelled_by(id)
                .on_hover_text("Name of the saved files, without the extension");
            ui.end_row();

            ui.label("Output size")
                .on_hover_text("Maximum side of the square atlas texture");
            let selected_size = match settings.size {
                Some(size) => format!("{size}×{size}"),
                None => AUTO_SIZE.to_owned(),
            };
            let min_size = state.build.min_size;
            MenuButton::from_button(Button::new(selected_size).frame_when_inactive(true)).ui(
                ui,
                |ui| {
                    let mut chosen = false;
                    if ui
                        .add(Button::new(AUTO_SIZE).selected(settings.size.is_none()))
                        .clicked()
                    {
                        settings.size = None;
                        chosen = true;
                    }
                    // A config can ask for a size the list does not have.
                    let custom = settings.size.filter(|size| !ATLAS_SIZES.contains(size));
                    for size in ATLAS_SIZES.into_iter().chain(custom) {
                        if ui
                            .add_enabled(
                                size >= min_size,
                                Button::new(format!("{size}×{size}"))
                                    .selected(settings.size == Some(size)),
                            )
                            .on_disabled_hover_text("Too small for the current images")
                            .clicked()
                        {
                            settings.size = Some(size);
                            chosen = true;
                        }
                    }
                    if chosen {
                        ui.close();
                    }
                },
            );
            ui.end_row();

            slider_field(
                ui,
                "Border padding",
                "Space in pixels around the whole atlas",
                &mut settings.border_padding,
                0,
                0..=10,
            );
            ui.end_row();
            slider_field(
                ui,
                "Texture padding",
                "Space in pixels between sprites",
                &mut settings.texture_padding,
                2,
                0..=10,
            );
            ui.end_row();

            let mut skip_metadata = settings.skip_serializing_metadata.unwrap_or_default();
            let id = ui
                .label("Skip metadata")
                .on_hover_text("Leave the metadata section out of the saved json")
                .id;
            if ui
                .checkbox(&mut skip_metadata, "")
                .labelled_by(id)
                .changed()
            {
                settings.skip_serializing_metadata = Some(skip_metadata);
            }
            ui.end_row();
        });
}

fn slider_field(
    ui: &mut Ui,
    label: &str,
    tooltip: &str,
    field: &mut Option<u32>,
    default_value: u32,
    range: RangeInclusive<u32>,
) {
    let id = ui.label(label).on_hover_text(tooltip).id;
    let mut value = field.unwrap_or(default_value);
    let changed = ui
        .horizontal(|ui| {
            let slider = Slider::new(&mut value, range.clone()).show_value(false);
            let slider = ui.add(slider).labelled_by(id);
            let number = DragValue::new(&mut value).range(range);
            let number = ui.add(number).labelled_by(id);
            slider.changed() || number.changed()
        })
        .inner;
    if changed {
        *field = Some(value);
    }
}

fn selection(ui: &mut Ui, state: &AppState) {
    let selected: Vec<&AppImageData> = state
        .project
        .image_data
        .iter()
        .filter(|image| state.ui.selected.contains(&image.path))
        .collect();
    match selected.as_slice() {
        [] => {
            ui.label(RichText::new("Nothing selected").color(HINT_TEXT));
        }
        [image] => sprite_details(ui, state, image),
        many => {
            ui.label(format!("{} sprites selected", many.len()));
        }
    }
}

fn sprite_details(ui: &mut Ui, state: &AppState, image: &AppImageData) {
    let position = state.build.output.as_ref().and_then(|built| {
        let frames = &built.atlas_asset.frames;
        let frame = frames.iter().find(|frame| frame.key == image.id)?;
        Some(format!("{}, {}", frame.frame.x, frame.frame.y))
    });
    Grid::new("selection_grid")
        .num_columns(2)
        .spacing(vec2(8.0, 4.0))
        .show(ui, |ui| {
            let row = |ui: &mut Ui, name: &str, value: &str| {
                ui.label(RichText::new(name).color(TEXT_WEAK));
                ui.add(Label::new(value).truncate()).on_hover_text(value);
                ui.end_row();
            };
            row(ui, "Key", image.id());
            row(ui, "Size", &format!("{}×{}", image.width, image.height));
            if let Some(position) = &position {
                row(ui, "Position", position);
            }
            row(ui, "Path", &image.path);
        });
}
