use std::path::PathBuf;

use egui::{
    CollapsingHeader, Color32, FontFamily, FontId, Grid, Image, Label, Layout, RichText,
    util::undoer::Undoer,
};
use rpack_cli::{ImageFile, Spritesheet, SpritesheetBuildConfig, SpritesheetError};
use texture_packer::TexturePackerConfig;

use crate::helpers::DroppedFileHelper;
pub const MY_ACCENT_COLOR32: Color32 = Color32::from_rgb(230, 102, 1);
pub const TOP_SIDE_MARGIN: f32 = 10.0;
pub const HEADER_HEIGHT: f32 = 45.0;
pub const TOP_BUTTON_WIDTH: f32 = 150.0;
pub const GIT_HASH: &str = env!("GIT_HASH");

pub struct Application {
    data: ApplicationData,
    output: Option<Spritesheet>,
    last_error: Option<SpritesheetError>,
    undoer: Undoer<ApplicationData>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
pub struct ApplicationData {
    #[serde(skip, default)]
    image_data: Vec<AppImageData>,
    #[serde(skip, default)]
    config: TexturePackerConfig,
    settings: Settings,
}
impl PartialEq for ApplicationData {
    fn eq(&self, other: &Self) -> bool {
        self.image_data == other.image_data
            && self.config.allow_rotation == other.config.allow_rotation
            && self.config.border_padding == other.config.border_padding
            && self.config.force_max_dimensions == other.config.force_max_dimensions
            && self.config.max_height == other.config.max_height
            && self.config.max_width == other.config.max_width
            && self.config.texture_extrusion == other.config.texture_extrusion
            && self.config.texture_outlines == other.config.texture_outlines
            && self.config.texture_padding == other.config.texture_padding
            && self.config.trim == other.config.trim
            && self.settings == other.settings
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct Settings {
    pub filename: String,
    pub size: u32,
    #[serde(skip)]
    min_size: [u32; 2],
    pub skip_metadata_serialization: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            filename: String::from("Tilemap"),
            size: 512,
            min_size: [32, 32],
            skip_metadata_serialization: false,
        }
    }
}

#[derive(Clone, PartialEq)]
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

    pub fn update_id(&mut self, prefix: &str) {
        self.data.id = self
            .path
            .strip_prefix(prefix)
            .unwrap_or(&self.path)
            .to_owned();
    }
}

impl Default for Application {
    fn default() -> Self {
        Self {
            data: Default::default(),
            undoer: Default::default(),
            output: None,
            last_error: None,
        }
    }
}

impl Application {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_config(&mut self, config: rpack_cli::TilemapGenerationConfig) {
        self.data.settings.size = config.size.unwrap_or(512);
        self.data.config = (&config).into();

        let (file_paths, prefix) = config.get_file_paths_and_prefix();
        self.data.image_data.clear();
        self.data
            .image_data
            .extend(file_paths.iter().flat_map(|f| f.create_image(&prefix)));
        self.rebuild_image_data();
    }
    pub fn get_common_prefix(&self) -> String {
        let file_paths: Vec<String> = self
            .data
            .image_data
            .iter()
            .map(|image| image.path.clone())
            .collect();
        rpack_cli::get_common_prefix(&file_paths)
    }
    pub fn rebuild_image_data(&mut self) {
        let prefix = self.get_common_prefix();
        self.data
            .image_data
            .iter_mut()
            .for_each(|f| f.update_id(prefix.as_str()));
        self.update_min_size();
    }
    pub fn update_min_size(&mut self) {
        self.data.settings.min_size[0] = self
            .data
            .image_data
            .iter()
            .max_by(|a, b| a.width.cmp(&b.width))
            .map_or(32, |s| s.width);
        self.data.settings.min_size[1] = self
            .data
            .image_data
            .iter()
            .max_by(|a, b| a.height.cmp(&b.height))
            .map_or(32, |s| s.height);
        for nr in [32, 64, 128, 256, 512, 1024, 2048, 4096] {
            if nr >= self.data.settings.min_size[0] && nr >= self.data.settings.min_size[1] {
                self.data.settings.min_size[0] = nr;
                self.data.settings.min_size[1] = nr;
                break;
            }
        }
    }
    /// Called once before the first frame.
    #[allow(dead_code, unused_variables, unused_mut)]
    pub fn new(cc: &eframe::CreationContext<'_>, config_file: Option<String>) -> Self {
        crate::fonts::setup_custom_fonts(&cc.egui_ctx);
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let mut app = Self::default();
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(config_file) = config_file {
            if let Ok(config) = rpack_cli::TilemapGenerationConfig::read_from_file(&config_file) {
                app.data.settings.filename = PathBuf::from(config_file)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .replace(".rpack_gen.json", "");
                app.read_config(config);
            }
        }

        app
    }

    fn build_atlas(&mut self, ctx: &egui::Context) {
        self.last_error = None;
        self.output = None;
        ctx.forget_image("bytes://output.png");
        if self.data.image_data.is_empty() {
            return;
        }
        let images: Vec<&ImageFile> = self.data.image_data.iter().map(|file| &file.data).collect();

        for multiplier in 1..10 {
            let size = multiplier * self.data.settings.min_size[0];
            if size > self.data.settings.size {
                break;
            }
            let config = TexturePackerConfig {
                max_width: size,
                max_height: size,
                ..self.data.config
            };
            match Spritesheet::build(
                SpritesheetBuildConfig {
                    packer_config: config,
                    skip_metadata_serialization: self.data.settings.skip_metadata_serialization,
                },
                &images,
                format!("{}.png", &self.data.settings.filename),
            ) {
                Ok(data) => {
                    let mut out_vec = vec![];
                    data.image_data
                        .write_to(
                            &mut std::io::Cursor::new(&mut out_vec),
                            image::ImageFormat::Png,
                        )
                        .unwrap();
                    ctx.include_bytes("bytes://output.png", out_vec);

                    self.output = Some(data);
                    break;
                }
                Err(e) => {
                    self.last_error = Some(e);
                }
            }
        }
        if self.output.is_some() {
            self.last_error = None;
        }
        ctx.request_repaint();
    }

    fn save_json(&self) -> Result<(), String> {
        let Some(spritesheet) = &self.output else {
            return Err("Data is incorrect".to_owned());
        };
        let data = spritesheet.atlas_asset_json.to_string();
        let filename = format!("{}.rpack.json", self.data.settings.filename);
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
        let Some(spritesheet) = &self.output else {
            return Err("Data is incorrect".to_owned());
        };
        let filename = format!("{}.png", self.data.settings.filename);
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
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.undoer
            .feed_state(ctx.input(|input| input.time), &self.data);
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
            if i.raw.dropped_files.is_empty() {
                return;
            }
            for file in i.raw.dropped_files.iter() {
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(path) = &file.path {
                    if path.is_dir() {
                        let Ok(dir) = path.read_dir() else {
                            continue;
                        };
                        for entry in dir {
                            if let Ok(entry) = entry {
                                let Ok(metadata) = entry.metadata() else {
                                    continue;
                                };
                                if metadata.is_file() {
                                    let Some(dyn_image) = entry.create_image("") else {
                                        continue;
                                    };
                                    self.data.image_data.push(dyn_image);
                                }
                            }
                        }
                    } else {
                        let Some(path) = &file.path else {
                            continue;
                        };
                        if path.to_string_lossy().ends_with(".rpack_gen.json") {
                            if let Ok(config) =
                                rpack_cli::TilemapGenerationConfig::read_from_file(&path)
                            {
                                self.data.settings.filename = path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .replace(".rpack_gen.json", "");
                                self.read_config(config);
                                break;
                            }
                        }
                    }
                }
                let Some(dyn_image) = file.create_image("") else {
                    continue;
                };
                self.data.image_data.push(dyn_image);
            }
            self.output = None;
            self.rebuild_image_data();
        });
        egui::TopBottomPanel::bottom("bottom_panel")
            .frame(egui::Frame::canvas(&ctx.style()))
            .show(ctx, |ui| {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.add_space(5.0);
                    ui.add_enabled_ui(self.undoer.has_undo(&self.data), |ui| {
                        if ui.button("⮪").on_hover_text("Go back").clicked() {
                            if let Some(action) = self.undoer.undo(&self.data) {
                                self.data = action.clone();
                                self.rebuild_image_data();
                                self.build_atlas(ui.ctx());
                            }
                        }
                    });
                    ui.add_enabled_ui(self.undoer.has_redo(&self.data), |ui| {
                        if ui.button("⮫").on_hover_text("Redo").clicked() {
                            if let Some(action) = self.undoer.redo(&self.data) {
                                self.data = action.clone();
                                self.rebuild_image_data();
                                self.build_atlas(ui.ctx());
                            }
                        }
                    });
                    ui.add_space(5.0);
                    powered_by_egui_and_eframe(ui);
                });
                ui.add_space(5.0);
            });
        egui::SidePanel::right("right")
            .min_width(200.0)
            .max_width(400.0)
            .frame(egui::Frame::canvas(&ctx.style()))
            .show_animated(ctx, !self.data.image_data.is_empty(), |ui| {
            egui::ScrollArea::vertical()
            .id_salt("rightPanel_scroll")
            .show(ui, |ui| {
                CollapsingHeader::new("Settings")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.vertical_centered_justified(|ui|{
                                let label = ui.label("Tilemap filename");
                                ui.text_edit_singleline(&mut self.data.settings.filename).labelled_by(label.id);
                                ui.add_space(10.0);
                            ui.add(
                                egui::Slider::new(&mut self.data.settings.size, self.data.settings.min_size[0]..=4096)
                                .step_by(32.0)
                                    .text("Max size"),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.data.config.border_padding, 0..=10)
                                    .text("Border Padding"),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.data.config.texture_padding, 0..=10)
                                    .text("Texture Padding"),
                            );
                            // ui.checkbox(&mut self.config.allow_rotation, "Allow Rotation")
                            // .on_hover_text("True to allow rotation of the input images. Default value is `true`. Images rotated will be rotated 90 degrees clockwise.");
                            ui.checkbox(&mut self.data.config.texture_outlines, "Texture Outlines")
            .on_hover_text("Draw the red line on the edge of the each frames. Useful for debugging.");
                            ui.checkbox(&mut self.data.settings.skip_metadata_serialization, "Skip Metadata Serialization")
            .on_hover_text("Skip metadata serialization.");
                            // ui.checkbox(&mut self.config.trim, "Trim").on_hover_text("True to trim the empty pixels of the input images.");
                            ui.add_space(10.0);

                            ui.add_enabled_ui(!self.data.image_data.is_empty(), |ui| {
                                    if ui
                                    .add_sized([TOP_BUTTON_WIDTH, 30.0], egui::Button::new("Build atlas"))
                                    .clicked()
                                    {
                                        self.build_atlas(ctx);
                                    }
                                    ui.add_space(10.0);

                                });
                            });
                    });
                ui.separator();
                CollapsingHeader::new("Image list")
                    .default_open(true)
                    .show_unindented(ui, |ui| {
                        ui.horizontal(|ui|{

                            if !self.data.image_data.is_empty() && ui.button("clear list").clicked() {
                                self.data.image_data.clear();
                                self.output = None;
                                self.update_min_size();
                            }
                            ui.add_space(10.0);
                            #[cfg(not(target_arch = "wasm32"))]
                            if ui.button("Add").clicked() {
                                if let Some(files) = rfd::FileDialog::new().set_title("Add images").add_filter("Images", &["png", "jpg", "jpeg","dds"]).pick_files(){
                                    for file in files.iter() {
                                        let Ok(image) = texture_packer::importer::ImageImporter::import_from_file(file) else { continue };
                                        let id = crate::helpers::id_from_path(&file.to_string_lossy());
                                        self.data.image_data.push(AppImageData { width: image.width(), height: image.height(), data: ImageFile { id: id, image }, path: file.to_string_lossy().to_string() });
                                    }
                                    self.rebuild_image_data();
                                }
                            }
                        });
                        let mut to_remove: Option<usize> = None;
                        let columns = if cfg!(target_arch = "wasm32") {
                            3
                        } else {
                            4
                        };
                        Grid::new("Image List").num_columns(columns).striped(true).spacing((10.0,10.0)).show(ui, |ui|{

                            for (index, file) in self.data.image_data.iter().enumerate() {
                                    if ui.button("x").clicked() {
                                        to_remove = Some(index);
                                    }
                                    #[cfg(not(target_arch = "wasm32"))]
                                    ui.image(format!("file://{}", file.path.as_str()));
                                    ui.add(Label::new(format!("{}x{}", file.width, file.height)).selectable(false));
                                    ui.add(Label::new(file.id()).selectable(false));
                                    ui.end_row();
                            }
                        });
                        if let Some(index) = to_remove {
                            self.data.image_data.remove(index);
                            self.output = None;
                            self.rebuild_image_data();
                        }
                    });
                });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = &self.last_error {
                let text = egui::RichText::new(format!("Error: {}", &error))
                    .font(FontId::new(20.0, FontFamily::Name("semibold".into())))
                    .color(Color32::RED)
                    .strong();
                ui.add(egui::Label::new(text));
            }
            egui::ScrollArea::vertical()
                .id_salt("vertical_scroll")
                .show(ui, |ui| {
                    if self.data.image_data.is_empty() {
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
                    let Some(data) = &self.output else {
                        return;
                    };
                    ui.vertical_centered_justified(|ui| {
                        egui::Frame::canvas(&ctx.style()).show(ui, |ui| {
                            ui.add_space(10.0);
                            ui.heading(
                                egui::RichText::new("Created atlas").color(MY_ACCENT_COLOR32),
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
                            ui.add(Image::from_uri("bytes://output.png"));
                            ui.separator();
                            ui.add_space(20.0);
                        });
                    });
                });
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.with_layout(Layout::right_to_left(egui::Align::Min), |ui| {
        ui.add_space(10.0);
        ui.hyperlink_to(format!("Build: {}", GIT_HASH), env!("CARGO_PKG_HOMEPAGE"));
        egui::warn_if_debug_build(ui);
        ui.separator();
        egui::widgets::global_theme_preference_switch(ui);
        ui.separator();
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.hyperlink_to("Mev Lyshkin", "https://www.mevlyshkin.com/");
        ui.add_space(10.0);
        ui.label("Made by ");
        ui.add_space((ui.available_width() - 10.0).max(15.0));
    });
}
