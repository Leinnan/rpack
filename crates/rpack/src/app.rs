use egui::{CollapsingHeader, Color32, DroppedFile, FontFamily, FontId, Image, RichText, Vec2};
use image::DynamicImage;
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
            name: String::from("Tilemap"),
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
    pub fn image_from_dropped_file<P>(file: &DroppedFile, prefix: P) -> Option<ImageFile>
    where
        P: AsRef<str>,
    {
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
        let base_id = id.replace(".png", "");

        let id = base_id
            .strip_prefix(prefix.as_ref())
            .unwrap_or(&base_id)
            .to_owned()
            .replace("\\", "/");

        let image = dynamic_image_from_file(file)?;
        Some(ImageFile { id, image })
    }

    fn build_atlas(&mut self, ctx: &egui::Context) {
        let prefix = Self::get_common_prefix(&self.dropped_files);
        println!("Prefix: {}", prefix);
        let images: Vec<ImageFile> = self
            .dropped_files
            .iter()
            .flat_map(|f| Self::image_from_dropped_file(f, &prefix))
            .collect();

        self.data = Some(Spritesheet::build(self.config, &images, "name"));
        if let Some(Ok(data)) = &self.data {
            let mut out_vec = vec![];
            let mut img =
                image::DynamicImage::new_rgba8(data.atlas_asset.size[0], data.atlas_asset.size[1]);
            image::imageops::overlay(&mut img, &data.image_data, 0, 0);

            img.write_to(
                &mut std::io::Cursor::new(&mut out_vec),
                image::ImageFormat::Png,
            )
            .unwrap();
            ctx.include_bytes("bytes://output.png", out_vec);
            self.image =
                Some(Image::from_uri("bytes://output.png").max_size(Vec2::new(256.0, 256.0)));
        }
        ctx.request_repaint();
    }

    fn save_atlas(&mut self) {
        let Some(Ok(data)) = &self.data else {
            return;
        };
        let data = data.image_data.as_bytes().to_vec();
        let filename = format!("{}.png", self.name);
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::io::Write;
            let path_buf = rfd::FileDialog::new()
                .set_directory(".")
                .add_filter("Image", &["png"])
                .set_file_name(filename)
                .save_file();
            if let Some(path) = path_buf {
                let mut file = std::fs::File::create(path).unwrap();
                let write_result = file.write_all(&data);
                if write_result.is_err() {
                    self.data = Some(Err(format!(
                        "Could not make atlas, error: {:?}",
                        write_result.unwrap_err()
                    )));
                } else {
                    println!("Output texture stored in {:?}", file);
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_directory(".")
                    .set_file_name(filename)
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
                                self.image = None;
                                ctx.forget_image("bytes://output.png");
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
            if let Some(Err(error)) = &self.data {
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
                                    ui.label("Tilemap id");
                                    ui.text_edit_singleline(&mut self.name);
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
                if let Some(Ok(data)) = &self.data {
                    ui.horizontal_top(|ui|{
                        ui.label(format!("{} frames, size: {}x{}",data.atlas_asset.frames.len(),data.atlas_asset.size[0],data.atlas_asset.size[1]));
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
