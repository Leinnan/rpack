use std::{collections::HashMap, path::Path};

use egui::{CollapsingHeader, Color32, DroppedFile, FontFamily, FontId, Image, RichText};
use image::{DynamicImage, GenericImageView};
use rpack_cli::{ImageFile, Spritesheet};
use texture_packer::{importer::ImageImporter, TexturePackerConfig};
pub const MY_ACCENT_COLOR32: Color32 = Color32::from_rgb(230, 102, 1);
pub const TOP_SIDE_MARGIN: f32 = 10.0;
pub const HEADER_HEIGHT: f32 = 45.0;
pub const TOP_BUTTON_WIDTH: f32 = 150.0;
pub const GIT_HASH: &str = env!("GIT_HASH");

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
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
    data: Option<Result<Spritesheet, String>>,
    #[serde(skip)]
    min_size: [u32; 2],
    #[serde(skip)]
    max_size: u32,
    #[serde(skip)]
    image_data: HashMap<String, ImageFile>,
}
impl Default for TemplateApp {
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

impl TemplateApp {
    pub fn rebuild_image_data(&mut self) {
        let prefix = Self::get_common_prefix(&self.dropped_files);
        self.image_data = self
            .dropped_files
            .iter()
            .flat_map(|f| Self::image_from_dropped_file(f, &prefix))
            .collect();
        self.update_min_size();
    }
    pub fn update_min_size(&mut self) {
        if let Some(file) = self
            .image_data
            .values()
            .max_by(|a, b| a.image.width().cmp(&b.image.width()))
        {
            self.min_size[0] = file.image.width();
        } else {
            self.min_size[0] = 32;
        }
        if let Some(file) = self
            .image_data
            .values()
            .max_by(|a, b| a.image.height().cmp(&b.image.height()))
        {
            self.min_size[1] = file.image.height();
        } else {
            self.min_size[1] = 32;
        }
    }
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);
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
    fn get_common_prefix(paths: &[DroppedFile]) -> String {
        if paths.is_empty() {
            return String::new();
        } else if paths.len() == 1 {
            let full_name = paths[0].file_path();
            let path = Path::new(&full_name)
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default();
            return full_name.strip_suffix(&path).unwrap_or_default().to_owned();
        }
        let mut prefix = paths[0].file_path();

        for s in paths.iter().skip(1) {
            let s = s.file_path();
            while !s.starts_with(&prefix) {
                prefix.pop(); // Remove the last character of the prefix
                if prefix.is_empty() {
                    return String::new();
                }
            }
        }

        prefix
    }
    pub fn image_from_dropped_file<P>(file: &DroppedFile, prefix: P) -> Option<(String, ImageFile)>
    where
        P: AsRef<str>,
    {
        let path = file.file_path();
        let base_id = path.replace(".png", "");

        let id = base_id
            .strip_prefix(prefix.as_ref())
            .unwrap_or(&base_id)
            .to_owned()
            .replace("\\", "/");

        let image: DynamicImage = dynamic_image_from_file(file)?;
        Some((path, ImageFile { id, image }))
    }

    fn build_atlas(&mut self, ctx: &egui::Context) {
        self.data = None;
        self.image = None;
        let images: Vec<ImageFile> = self.image_data.values().cloned().collect();

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

fn setup_custom_fonts(ctx: &egui::Context) {
    // Start with the default fonts (we will be adding to them rather than replacing them).
    let mut fonts = egui::FontDefinitions::default();

    // Install my own font (maybe supporting non-latin characters).
    // .ttf and .otf files supported.
    fonts.font_data.insert(
        "regular".to_owned(),
        egui::FontData::from_static(include_bytes!("../static/JetBrainsMonoNL-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "semibold".to_owned(),
        egui::FontData::from_static(include_bytes!("../static/JetBrainsMono-SemiBold.ttf")).into(),
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

impl eframe::App for TemplateApp {
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
                    ui.add(egui::Label::new(text));
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
        egui::SidePanel::right("leftPanel")
            .min_width(200.0)
            .frame(egui::Frame::canvas(&ctx.style()))
            .show_animated(ctx, !self.image_data.is_empty(), |ui| {
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
                        if !self.image_data.is_empty() && ui.button("clear list").clicked() {
                            self.image_data.clear();
                            self.dropped_files.clear();
                            self.data = None;
                            self.update_min_size();
                        }
                        let mut to_remove: Option<String> = None;
                        for (id, file) in self.image_data.iter() {
                            ui.horizontal_top(|ui| {
                                ui.add_space(10.0);
                                if ui.button("x").clicked() {
                                    to_remove = Some(id.clone());
                                }
                                ui.add_space(10.0);
                                let (x, y) = file.image.dimensions();
                                ui.label(&file.id)
                                    .on_hover_text(format!("Dimensions: {}x{}", x, y));
                            });
                        }
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
                            ui.label(
                                RichText::new("Drop files here")
                                    .heading()
                                    .color(MY_ACCENT_COLOR32),
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
                            ui.add_enabled_ui(self.data.is_some(), |ui| {
                                if ui
                                    .add_sized(
                                        [TOP_BUTTON_WIDTH, 30.0],
                                        egui::Button::new("Save atlas image"),
                                    )
                                    .clicked()
                                {
                                    if let Err(error) = self.save_atlas() {
                                        eprintln!("ERROR: {}", error);
                                    }
                                }
                            });
                            ui.add_space(10.0);
                            CollapsingHeader::new("Atlas JSON")
                                .default_open(true)
                                .show(ui, |ui| {
                                    ui.vertical_centered_justified(|ui| {
                                        if ui
                                            .add(egui::Button::new("Copy JSON to Clipboard"))
                                            .clicked()
                                        {
                                            ui.output_mut(|o| {
                                                o.copied_text = data.atlas_asset_json.to_string()
                                            });
                                        };
                                        ui.add_space(10.0);
                                        ui.label(RichText::new("Frames JSON").strong());
                                        ui.add_space(10.0);
                                        egui_json_tree::JsonTree::new(
                                            "simple-tree",
                                            &data.atlas_asset_json,
                                        )
                                        .show(ui);
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

trait FilePath {
    fn file_path(&self) -> String;
}

impl FilePath for DroppedFile {
    fn file_path(&self) -> String {
        let id;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = self.path.as_ref().unwrap().clone();
            id = path.to_str().unwrap().to_owned();
        }
        #[cfg(target_arch = "wasm32")]
        {
            id = self.name.clone();
        }
        id.replace(".png", "")
    }
}

fn dynamic_image_from_file(file: &DroppedFile) -> Option<DynamicImage> {
    #[cfg(target_arch = "wasm32")]
    {
        let bytes = file.bytes.as_ref().clone()?;

        if let Ok(r) = ImageImporter::import_from_memory(bytes) {
            Some(r.into())
        } else {
            None
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = file.path.as_ref()?;

        if let Ok(r) = ImageImporter::import_from_file(path) {
            Some(r)
        } else {
            None
        }
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
        ui.label(". ");
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
