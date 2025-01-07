use std::{collections::HashMap, io::Cursor};

use egui::{CollapsingHeader, Color32, DroppedFile, FontFamily, FontId, Image, RichText, Vec2};
use image::DynamicImage;
use serde_json::Value;
use texture_packer::{importer::ImageImporter, TexturePacker, TexturePackerConfig};
pub const MY_ACCENT_COLOR32: Color32 = Color32::from_rgb(230, 102, 1);
pub const TOP_SIDE_MARGIN: f32 = 10.0;
pub const HEADER_HEIGHT: f32 = 45.0;
pub const TOP_BUTTON_WIDTH: f32 = 150.0;
pub const GIT_HASH: &str = env!("GIT_HASH");

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AtlasFrame {
    pub key: String,
    pub frame: SerializableRect,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AtlasAsset {
    pub size: [u32; 2],
    pub name: String,
    pub frames: Vec<AtlasFrame>,
}

#[derive(Clone)]
pub struct Spritesheet {
    pub data: Vec<u8>,
    pub frames: HashMap<String, texture_packer::Frame<String>>,
    pub atlas_asset_json: Value,
    pub size: (u32, u32),
}

/// Boundaries and properties of a packed texture.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct SerializableFrame {
    /// Key used to uniquely identify this frame.
    pub key: String,
    /// Rectangle describing the texture coordinates and size.
    pub frame: SerializableRect,
    /// True if the texture was rotated during packing.
    /// If it was rotated, it was rotated 90 degrees clockwise.
    pub rotated: bool,
    /// True if the texture was trimmed during packing.
    pub trimmed: bool,

    // (x, y) is the trimmed frame position at original image
    // (w, h) is original image size
    //
    //            w
    //     +--------------+
    //     | (x, y)       |
    //     |  ^           |
    //     |  |           |
    //     |  *********   |
    //     |  *       *   |  h
    //     |  *       *   |
    //     |  *********   |
    //     |              |
    //     +--------------+
    /// Source texture size before any trimming.
    pub source: SerializableRect,
}

impl From<texture_packer::Frame<String>> for SerializableFrame {
    fn from(value: texture_packer::Frame<String>) -> Self {
        SerializableFrame {
            key: value.key,
            frame: value.frame.into(),
            rotated: value.rotated,
            trimmed: value.trimmed,
            source: value.source.into(),
        }
    }
}

/// Defines a rectangle in pixels with the origin at the top-left of the texture atlas.
#[derive(Copy, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct SerializableRect {
    /// Horizontal position the rectangle begins at.
    pub x: u32,
    /// Vertical position the rectangle begins at.
    pub y: u32,
    /// Width of the rectangle.
    pub w: u32,
    /// Height of the rectangle.
    pub h: u32,
}

impl From<texture_packer::Rect> for SerializableRect {
    fn from(value: texture_packer::Rect) -> Self {
        SerializableRect {
            h: value.h,
            w: value.w,
            x: value.x,
            y: value.y,
        }
    }
}

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
    counter: i32,
    #[serde(skip)]
    data: Option<Spritesheet>,
    #[serde(skip)]
    error: Option<String>,
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
                ..Default::default()
            },
            counter: 0,
            image: None,
            data: None,
            error: None,
        }
    }
}

impl TemplateApp {
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
        }
        let mut prefix = file_path(&paths[0]);

        for s in paths.iter().skip(1) {
            let s = file_path(s);
            while !s.starts_with(&prefix) {
                prefix.pop(); // Remove the last character of the prefix
                if prefix.is_empty() {
                    return String::new();
                }
            }
        }

        prefix
    }

    fn build_atlas(&mut self, ctx: &egui::Context) {
        self.error = None;
        let mut packer = TexturePacker::new_skyline(self.config);
        let prefix = Self::get_common_prefix(&self.dropped_files);
        println!("Prefix: {}", prefix);
        for file in &self.dropped_files {
            let base_id = file_path(file);
            let id = base_id
                .strip_prefix(&prefix)
                .unwrap_or(&base_id)
                .to_owned()
                .replace("\\", "/");
            println!("Base id: {}, ID: {}", &base_id, &id);
            let texture = dynamic_image_from_file(file);
            let can_pack = packer.can_pack(&texture);

            if can_pack {
                packer.pack_own(id, texture).unwrap();
            } else {
                self.error = Some(format!(
                    "Consider making atlas bigger. Could not make atlas, failed on: {}",
                    id
                ));
                return;
            }
        }
        for (name, frame) in packer.get_frames() {
            println!("  {:7} : {:?}", name, frame.frame);
        }
        let mut out_vec = vec![];
        let exported_image = texture_packer::exporter::ImageExporter::export(&packer).unwrap();
        let mut img = image::DynamicImage::new_rgba8(self.config.max_width, self.config.max_height);
        image::imageops::overlay(&mut img, &exported_image, 0, 0);

        img.write_to(&mut Cursor::new(&mut out_vec), image::ImageFormat::Png)
            .unwrap();
        let atlas = AtlasAsset {
            size: [img.width(), img.height()],
            name: "Atlas".to_owned(),
            frames: packer
                .get_frames()
                .values()
                .map(|v| -> AtlasFrame {
                    AtlasFrame {
                        key: v.key.clone(),
                        frame: SerializableRect {
                            x: v.frame.x,
                            y: v.frame.y,
                            w: v.frame.w,
                            h: v.frame.h,
                        },
                    }
                })
                .collect(),
        };
        let frames_string = serde_json::to_string_pretty(&atlas).unwrap();

        let atlas_asset_json = serde_json::from_str(&frames_string).unwrap();
        self.data = Some(Spritesheet {
            data: out_vec.clone(),
            frames: packer.get_frames().clone(),
            size: (img.width(), img.height()),
            atlas_asset_json,
        });
        let id = format!("bytes://output_{}.png", self.counter);
        self.image = None;
        ctx.forget_image(&id);
        self.counter += 1;

        let id = format!("bytes://output_{}.png", self.counter);
        ctx.include_bytes(id.clone(), out_vec.clone());
        self.image = Some(Image::from_uri(id.clone()).max_size(Vec2::new(256.0, 256.0)));
        ctx.request_repaint();
    }

    fn save_atlas(&mut self) {
        if self.data.is_none() {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let data = self.data.clone().unwrap().data;
            use std::io::Write;
            let path_buf = rfd::FileDialog::new()
                .set_directory(".")
                .add_filter("Image", &["png"])
                .set_file_name("output.png")
                .save_file();
            if let Some(path) = path_buf {
                let mut file = std::fs::File::create(path).unwrap();
                let write_result = file.write_all(&data);
                if write_result.is_err() {
                    self.error = Some(format!(
                        "Could not make atlas, error: {:?}",
                        write_result.unwrap_err()
                    ));
                } else {
                    println!("Output texture stored in {:?}", file);
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let data = self.data.clone().unwrap().data;
            wasm_bindgen_futures::spawn_local(async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_directory(".")
                    .set_file_name("output.png")
                    .save_file()
                    .await;
                match file {
                    None => (),
                    Some(file) => {
                        // let module = serde_yaml::to_string(&module).unwrap();
                        // TODO: error handling
                        file.write(&data).await.unwrap();
                    }
                }
            });
        }
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
                ui.with_layout(
                    egui::Layout::left_to_right(egui::Align::Center)
                        .with_cross_align(eframe::emath::Align::Center),
                    |ui| {
                        let text = egui::RichText::new("rPack")
                            .font(FontId::new(26.0, FontFamily::Name("semibold".into())))
                            .color(MY_ACCENT_COLOR32)
                            .strong();
                        ui.allocate_space(egui::vec2(TOP_SIDE_MARGIN, HEADER_HEIGHT));
                        ui.add(egui::Label::new(text));
                        let available_width =
                            ui.available_width() - ((TOP_BUTTON_WIDTH - TOP_SIDE_MARGIN) * 3.0);
                        ui.allocate_space(egui::vec2(available_width, HEADER_HEIGHT));
                        ui.add_enabled_ui(self.data.is_some(), |ui| {
                            if ui
                                .add_sized([TOP_BUTTON_WIDTH, 30.0], egui::Button::new("Save"))
                                .clicked()
                            {
                                self.save_atlas();
                            }
                        });
                        ui.add_enabled_ui(!self.dropped_files.is_empty(), |ui| {
                            ui.allocate_space(egui::vec2(TOP_SIDE_MARGIN, 10.0));
                            if ui
                                .add_sized(
                                    [TOP_BUTTON_WIDTH, 30.0],
                                    egui::Button::new("Build atlas"),
                                )
                                .clicked()
                            {
                                self.build_atlas(ctx);
                            }
                        });
                        ui.allocate_space(egui::vec2(TOP_SIDE_MARGIN, 10.0));
                    },
                );
            });
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let mut extra = i.raw.dropped_files.clone();
                self.dropped_files.append(&mut extra);
            }
        });
        egui::TopBottomPanel::bottom("bottom_panel")
            .frame(egui::Frame::canvas(&ctx.style()))
            .show(ctx, |ui| {
                powered_by_egui_and_eframe(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = &self.error {
                let text = egui::RichText::new(format!("Error: {}",&error))
                .font(FontId::new(20.0, FontFamily::Name("semibold".into())))
                .color(Color32::RED)
                .strong();
                ui.add(egui::Label::new(text));
            }
            if !self.dropped_files.is_empty() {
                ui.horizontal_top(|ui|{
                        if let Some(image) = &self.image {
                            ui.add(image.clone());
                        }
                        CollapsingHeader::new("Settings")
                                .default_open(false)
                                .show(ui, |ui| {
                        ui.add(
                            egui::Slider::new(&mut self.config.max_width, 64..=4096).text("Width"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.config.max_height, 64..=4096).text("Height"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.config.border_padding, 0..=10).text("Border Padding"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.config.texture_padding, 0..=10).text("Texture Padding"),
                        );
                        ui.checkbox(&mut self.config.allow_rotation, "Allow Rotation")
                        .on_hover_text("True to allow rotation of the input images. Default value is `true`. Images rotated will be rotated 90 degrees clockwise.");
                        ui.checkbox(&mut self.config.texture_outlines, "Texture Outlines")
                        .on_hover_text("True to draw the red line on the edge of the each frames. Useful for debugging.");
                        ui.checkbox(&mut self.config.trim, "Trim").on_hover_text("True to trim the empty pixels of the input images.");
                                });
                });
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui|{

                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                if let Some(data) = &self.data {
                    ui.horizontal_top(|ui|{
                        ui.label(format!("{} frames, size: {}x{}",data.frames.len(),data.size.0,data.size.1));
                    });
                    ui.label(RichText::new("Frames JSON").strong());
                    egui_json_tree::JsonTree::new("simple-tree", &data.atlas_asset_json).show(ui);
                    if ui
                    .add(egui::Button::new("Copy JSON to Clipboard"))
                    .clicked()
                {
                    ui.output_mut(|o| o.copied_text = data.atlas_asset_json.to_string());
                };
                }
                ui.separator();
                    let mut index_to_remove : Option<usize> = None;
                    for (i, file) in self.dropped_files.iter().enumerate() {
                        let mut info = if let Some(path) = &file.path {
                            path.display().to_string()
                        } else if !file.name.is_empty() {
                            file.name.clone()
                        } else {
                            "???".to_owned()
                        };
                        if let Some(bytes) = &file.bytes {
                            info += &format!(" ({} bytes)", bytes.len());
                        }
                        ui.horizontal_top(|ui|{
                            if ui.button("x").clicked(){
                                index_to_remove = Some(i);
                            }
                            ui.add_space(10.0);
                            ui.label(info);
                        });
                    }
                    if let Some(index) = index_to_remove{
                        self.dropped_files.remove(index);
                    }
                });
                if ui.button("clear list").clicked() {
                    self.dropped_files.clear();
                }
                });
            } else {
                ui.vertical_centered_justified(|ui|{
                    ui.add_space(50.0);
                    ui.label(
                        RichText::new("Drop files here")
                            .heading()
                            .color(MY_ACCENT_COLOR32),
                    );

                });
            }
        });
    }
}

fn file_path(file: &DroppedFile) -> String {
    let id;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = file.path.as_ref().unwrap().clone();
        id = path.to_str().unwrap().to_owned();
    }
    #[cfg(target_arch = "wasm32")]
    {
        id = file.name.clone();
    }
    id.replace(".png", "")
}

fn dynamic_image_from_file(file: &DroppedFile) -> DynamicImage {
    #[cfg(target_arch = "wasm32")]
    {
        let bytes = file.bytes.as_ref().clone();

        ImageImporter::import_from_memory(&bytes.unwrap())
            .expect("Unable to import file. Run this example with --features=\"png\"")
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = file.path.as_ref().unwrap().clone();

        ImageImporter::import_from_file(&path)
            .expect("Unable to import file. Run this example with --features=\"png\"")
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
