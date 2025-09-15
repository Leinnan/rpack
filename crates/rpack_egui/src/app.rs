use std::collections::HashMap;

use egui::{
    CollapsingHeader, Color32, DroppedFile, FontFamily, FontId, Grid, Image, Label, RichText,
};
use rpack_cli::{ImageFile, Spritesheet, SpritesheetError};
use texture_packer::TexturePackerConfig;

use crate::helpers::DroppedFileHelper;
pub const MY_ACCENT_COLOR32: Color32 = Color32::from_rgb(230, 102, 1);
pub const TOP_SIDE_MARGIN: f32 = 10.0;
pub const HEADER_HEIGHT: f32 = 45.0;
pub const TOP_BUTTON_WIDTH: f32 = 150.0;
pub const GIT_HASH: &str = env!("GIT_HASH");

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct Application {
    #[serde(skip)]
    dropped_files: Vec<DroppedFile>,
    #[serde(skip)]
    config: TexturePackerConfig,
    #[serde(skip)]
    image: Option<Image<'static>>,
    #[serde(skip)]
    name: String,
    #[serde(skip)]
    counter: i32,
    #[serde(skip)]
    data: Option<Result<Spritesheet, SpritesheetError>>,
    #[serde(skip)]
    min_size: [u32; 2],
    #[serde(skip)]
    max_size: u32,
    #[serde(skip)]
    image_data: HashMap<String, AppImageData>,
}

pub struct AppImageData {
    pub width: u32,
    pub height: u32,
    pub data: ImageFile,
    pub path: String,
}

impl AppImageData {
    pub fn id(&self) -> &str {
        self.data.id.as_str()
    }
}

impl Default for Application {
    fn default() -> Self {
        Self {
            dropped_files: vec![],
            config: TexturePackerConfig {
                max_width: 512,
                max_height: 512,
                allow_rotation: false,
                border_padding: 2,
                trim: false,
                force_max_dimensions: true,
                ..Default::default()
            },
            counter: 0,
            image: None,
            data: None,
            max_size: 4096,
            name: String::from("Tilemap"),
            min_size: [32, 32],
            image_data: HashMap::new(),
        }
    }
}

impl Application {
    pub fn rebuild_image_data(&mut self) {
        let file_paths: Vec<String> = self
            .dropped_files
            .iter()
            .map(|dropped_file| dropped_file.file_path())
            .collect();
        let prefix = rpack_cli::get_common_prefix(&file_paths);

        self.image_data = self
            .dropped_files
            .iter()
            .flat_map(|f| f.create_image(&prefix).map(|i| (i.id().to_string(), i)))
            .collect();
        self.update_min_size();
    }
    pub fn update_min_size(&mut self) {
        if let Some(file) = self
            .image_data
            .values()
            .max_by(|a, b| a.width.cmp(&b.width))
        {
            self.min_size[0] = file.width;
        } else {
            self.min_size[0] = 32;
        }
        if let Some(file) = self
            .image_data
            .values()
            .max_by(|a, b| a.height.cmp(&b.height))
        {
            self.min_size[1] = file.height;
        } else {
            self.min_size[1] = 32;
        }
    }
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::fonts::setup_custom_fonts(&cc.egui_ctx);
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    fn build_atlas(&mut self, ctx: &egui::Context) {
        self.data = None;
        self.image = None;
        let images: Vec<ImageFile> = self
            .image_data
            .values()
            .map(|file| file.data.clone())
            .collect();

        for size in [32, 64, 128, 256, 512, 1024, 2048, 4096] {
            if size < self.min_size[0] || size < self.min_size[1] {
                continue;
            }
            if size > self.max_size {
                break;
            }
            let config = TexturePackerConfig {
                max_width: size,
                max_height: size,
                ..self.config
            };
            self.data = Some(Spritesheet::build(
                config,
                &images,
                format!("{}.png", &self.name),
            ));
            if let Some(Ok(data)) = &self.data {
                let mut out_vec = vec![];
                data.image_data
                    .write_to(
                        &mut std::io::Cursor::new(&mut out_vec),
                        image::ImageFormat::Png,
                    )
                    .unwrap();
                ctx.include_bytes("bytes://output.png", out_vec);
                self.image = Some(Image::from_uri("bytes://output.png"));
                break;
            }
        }
        ctx.request_repaint();
    }

    fn save_json(&self) -> Result<(), String> {
        let Some(Ok(spritesheet)) = &self.data else {
            return Err("Data is incorrect".to_owned());
        };
        let data = spritesheet.atlas_asset_json.to_string();
        let filename = format!("{}.rpack.json", self.name);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path_buf = rfd::FileDialog::new()
                .set_directory(".")
                .add_filter(".rpack.json", &["rpack.json"])
                .set_file_name(filename)
                .save_file();
            if let Some(path) = path_buf {
                let write_result = std::fs::write(path, &data);
                if write_result.is_err() {
                    return Err(format!(
                        "Could not save json atlas, error: {:?}",
                        write_result.unwrap_err()
                    ));
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                let Some(file) = rfd::AsyncFileDialog::new()
                    .set_directory(".")
                    .set_file_name(filename)
                    .save_file()
                    .await
                else {
                    return;
                };
                file.write(data.as_bytes()).await.unwrap();
            });
        }
        Ok(())
    }

    fn save_atlas(&self) -> Result<(), String> {
        let Some(Ok(spritesheet)) = &self.data else {
            return Err("Data is incorrect".to_owned());
        };
        let filename = format!("{}.png", self.name);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path_buf = rfd::FileDialog::new()
                .set_directory(".")
                .add_filter("png", &["png"])
                .set_file_name(filename)
                .save_file();
            if let Some(path) = path_buf {
                let write_result = spritesheet
                    .image_data
                    .save_with_format(path, image::ImageFormat::Png);
                if write_result.is_err() {
                    return Err(format!(
                        "Could not make atlas, error: {:?}",
                        write_result.unwrap_err()
                    ));
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mut data = vec![];
            let Ok(()) = spritesheet.image_data.write_to(
                &mut std::io::Cursor::new(&mut data),
                image::ImageFormat::Png,
            ) else {
                return Err("Failed to copy data".to_owned());
            };
            wasm_bindgen_futures::spawn_local(async move {
                let Some(file) = rfd::AsyncFileDialog::new()
                    .set_directory(".")
                    .set_file_name(filename)
                    .save_file()
                    .await
                else {
                    return;
                };
                file.write(&data).await.unwrap();
            });
        }
        Ok(())
    }
}

impl eframe::App for Application {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.dropped_files.is_empty() && self.image.is_some() {
            self.image = None;
            self.data = None;
        }
        egui::TopBottomPanel::top("topPanel")
            .frame(egui::Frame::canvas(&ctx.style()))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let text = egui::RichText::new("rPack")
                        .font(FontId::new(26.0, FontFamily::Name("semibold".into())))
                        .color(MY_ACCENT_COLOR32)
                        .strong();
                    ui.allocate_space(egui::vec2(TOP_SIDE_MARGIN, HEADER_HEIGHT));
                    ui.add(egui::Label::new(text).selectable(false));
                });
            });
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let mut extra = i.raw.dropped_files.clone();
                self.dropped_files.append(&mut extra);
                self.data = None;
                self.rebuild_image_data();
            }
        });
        egui::TopBottomPanel::bottom("bottom_panel")
            .frame(egui::Frame::canvas(&ctx.style()))
            .show(ctx, |ui| {
                powered_by_egui_and_eframe(ui);
            });
        egui::SidePanel::right("right")
            .min_width(200.0)
            .max_width(400.0)
            .frame(egui::Frame::canvas(&ctx.style()))
            .show_animated(ctx, !self.image_data.is_empty(), |ui| {
            egui::ScrollArea::vertical()
            .id_salt("rightPanel_scroll")
            .show(ui, |ui| {
                CollapsingHeader::new("Settings")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.vertical_centered_justified(|ui|{
                                let label = ui.label("Tilemap filename");
                                ui.text_edit_singleline(&mut self.name).labelled_by(label.id);
                                ui.add_space(10.0);
                            ui.add(
                                egui::Slider::new(&mut self.max_size, self.min_size[0]..=4096)
                                .step_by(32.0)
                                    .text("Max size"),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.config.border_padding, 0..=10)
                                    .text("Border Padding"),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.config.texture_padding, 0..=10)
                                    .text("Texture Padding"),
                            );
                            // ui.checkbox(&mut self.config.allow_rotation, "Allow Rotation")
                            // .on_hover_text("True to allow rotation of the input images. Default value is `true`. Images rotated will be rotated 90 degrees clockwise.");
                            ui.checkbox(&mut self.config.texture_outlines, "Texture Outlines")
            .on_hover_text("Draw the red line on the edge of the each frames. Useful for debugging.");
                            // ui.checkbox(&mut self.config.trim, "Trim").on_hover_text("True to trim the empty pixels of the input images.");
                            ui.add_space(10.0);

                            ui.add_enabled_ui(!self.dropped_files.is_empty(), |ui| {
                                    if ui
                                    .add_sized([TOP_BUTTON_WIDTH, 30.0], egui::Button::new("Build atlas"))
                                    .clicked()
                                    {
                                        self.image = None;
                                        ctx.forget_image("bytes://output.png");
                                        self.build_atlas(ctx);
                                    }
                                    ui.add_space(10.0);

                                });
                            });
                    });
                ui.separator();
                CollapsingHeader::new("Image list")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui|{

                            if !self.image_data.is_empty() && ui.button("clear list").clicked() {
                                self.image_data.clear();
                                self.dropped_files.clear();
                                self.data = None;
                                self.update_min_size();
                            }
                            ui.add_space(10.0);
                            #[cfg(not(target_arch = "wasm32"))]
                            if ui.button("Add").clicked() {
                                if let Some(files) = rfd::FileDialog::new().set_title("Add images").add_filter("Images", &["png", "jpg", "jpeg","dds"]).pick_files(){
                                    for file in files.iter() {
                                        let Ok(image) = texture_packer::importer::ImageImporter::import_from_file(file) else { continue };
                                        let id = crate::helpers::id_from_path(&file.to_string_lossy());
                                        self.image_data.insert(file.to_string_lossy().to_string(), AppImageData { width: image.width(), height: image.height(), data: ImageFile { id: id, image }, path: file.to_string_lossy().to_string() });
                                    }
                                    self.update_min_size();
                                }
                            }
                        });
                        let mut to_remove: Option<String> = None;
                        Grid::new("Image List").num_columns(4).striped(true).spacing((10.0,10.0)).show(ui, |ui|{

                            for (id, file) in self.image_data.iter() {
                                    if ui.button("x").clicked() {
                                        to_remove = Some(id.clone());
                                    }

                                    ui.image(format!("file://{}", file.path));
                                    ui.add(Label::new(file.id()).selectable(false));
                                    ui.add(Label::new(format!("{}x{}", file.width, file.height)).selectable(false));
                                    ui.end_row();
                            }
                        });
                        if let Some(index) = to_remove {
                            if let Some(i) = self
                                .dropped_files
                                .iter()
                                .position(|e| e.file_path().eq(&index))
                            {
                                self.dropped_files.remove(i);
                                self.image_data.remove(&index);
                                self.data = None;
                                self.rebuild_image_data();
                            }
                        }
                    });
                });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("vertical_scroll")
                .show(ui, |ui| {
                    if let Some(Err(error)) = &self.data {
                        let text = egui::RichText::new(format!("Error: {}", &error))
                            .font(FontId::new(20.0, FontFamily::Name("semibold".into())))
                            .color(Color32::RED)
                            .strong();
                        ui.add(egui::Label::new(text));
                        return;
                    }
                    if self.dropped_files.is_empty() {
                        ui.vertical_centered_justified(|ui| {
                            ui.add_space(50.0);
                            ui.add(
                                Label::new(
                                    RichText::new("Drop images here first")
                                        .heading()
                                        .color(MY_ACCENT_COLOR32),
                                )
                                .selectable(false),
                            );
                        });
                    }
                    let Some(image) = &self.image else {
                        return;
                    };
                    let Some(Ok(data)) = &self.data else {
                        return;
                    };
                    ui.vertical_centered_justified(|ui| {
                        egui::Frame::canvas(&ctx.style()).show(ui, |ui| {
                            ui.add_space(10.0);
                            ui.heading(
                                egui::RichText::new("Crated atlas").color(MY_ACCENT_COLOR32),
                            );
                            ui.add_space(10.0);
                            ui.label(format!(
                                "{} sprites\nsize: {}x{}",
                                data.atlas_asset.frames.len(),
                                data.atlas_asset.size[0],
                                data.atlas_asset.size[1]
                            ));
                            ui.add_space(10.0);
                            ui.horizontal_wrapped(|ui| {
                                let width = (ui.available_width() - 30.0).max(1.0) / 2.0;
                                ui.add_space(10.0);
                                if ui
                                    .add_sized([width, 35.0], egui::Button::new("Save atlas image"))
                                    .clicked()
                                {
                                    if let Err(error) = self.save_atlas() {
                                        eprintln!("ERROR: {}", error);
                                    }
                                }
                                ui.add_space(10.0);
                                if ui
                                    .add_sized([width, 35.0], egui::Button::new("Save atlas json"))
                                    .clicked()
                                {
                                    if let Err(error) = self.save_json() {
                                        eprintln!("ERROR: {}", error);
                                    }
                                }
                                ui.add_space(10.0);
                            });
                            ui.add_space(10.0);
                            CollapsingHeader::new("Atlas JSON")
                                .default_open(true)
                                .show(ui, |ui| {
                                    ui.vertical_centered_justified(|ui| {
                                        ui.add_space(10.0);
                                        egui_json_tree::JsonTree::new(
                                            "simple-tree",
                                            &data.atlas_asset_json,
                                        )
                                        .show(ui);
                                        #[cfg(not(target_arch = "wasm32"))]
                                        {
                                            ui.add_space(10.0);
                                            if ui
                                                .add(egui::Button::new("Copy JSON to Clipboard"))
                                                .clicked()
                                            {
                                                ui.ctx()
                                                    .copy_text(data.atlas_asset_json.to_string());
                                            };
                                        }
                                    });
                                });
                            ui.add_space(10.0);
                            ui.separator();
                            ui.add(image.clone());
                            ui.separator();
                            ui.add_space(10.0);
                            ui.add_space(10.0);
                        });
                    });
                });
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.hyperlink_to(format!("Build: {}", GIT_HASH), env!("CARGO_PKG_HOMEPAGE"));
        egui::warn_if_debug_build(ui);
        ui.separator();
        egui::widgets::global_theme_preference_switch(ui);
        ui.separator();
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Made by ");
        ui.hyperlink_to("Mev Lyshkin", "https://www.mevlyshkin.com/");
    });
}
