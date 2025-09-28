use crossbeam::queue::SegQueue;
use egui::containers::menu::MenuButton;
use egui::{
    util::undoer::Undoer, Button, Color32, FontFamily, FontId, Frame, Image, Label, Layout,
    RichText, Slider, Ui,
};
use egui::{Checkbox, Grid, Vec2};
use egui_extras::{Column, TableBuilder};
use once_cell::sync::Lazy;
use rpack_cli::TilemapGenerationConfig;
use rpack_cli::{
    packer::SkylinePacker, ImageFile, Spritesheet, SpritesheetBuildConfig, SpritesheetError,
};
use texture_packer::{Rect, TexturePackerConfig};

use crate::helpers::DroppedFileHelper;
use crate::view_settings::ViewSettings;
static INPUT_QUEUE: Lazy<SegQueue<AppImageAction>> = Lazy::new(SegQueue::new);
pub const MY_ACCENT_COLOR32: Color32 = Color32::from_rgb(230, 102, 1);
pub const GIT_HASH: &str = env!("GIT_HASH");
pub const VERSION: &str = concat!(" v ", env!("CARGO_PKG_VERSION"), " ");

pub const ICON_DATA: &[u8] = include_bytes!("../static/base_icon.png");

#[derive(Clone)]
#[allow(dead_code)]
pub enum AppImageAction {
    Add(AppImageData),
    Replace(Vec<AppImageData>),
    Remove(usize),
    UpdateSpriteSheet(Result<Spritesheet, SpritesheetError>),
    Clear,
    RebuildAtlas,
    #[cfg(not(target_arch = "wasm32"))]
    ReadFromConfig(TilemapGenerationConfig, PathBuf),
}

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::{future::Future, ops::RangeInclusive};

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    // this is stupid... use any executor of your choice instead
    std::thread::spawn(move || futures::executor::block_on(f));
}
#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

pub enum SpriteSheetState {
    Empty,
    Building,
    Ok(Spritesheet),
}
impl SpriteSheetState {
    pub fn is_ok(&self) -> bool {
        matches!(self, SpriteSheetState::Ok(_))
    }
    pub fn is_none(&self) -> bool {
        matches!(self, SpriteSheetState::Empty)
    }
    pub fn is_building(&self) -> bool {
        matches!(self, SpriteSheetState::Building)
    }
}

pub struct Application {
    data: ApplicationData,
    output: SpriteSheetState,
    last_error: Option<SpritesheetError>,
    undoer: Undoer<ApplicationData>,
    last_editor_paths: Vec<String>,
    view_settings: ViewSettings,
    show_modal: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone, PartialEq)]
pub struct ApplicationData {
    #[serde(skip, default)]
    image_data: Vec<AppImageData>,
    #[serde(skip, default)]
    settings: TilemapGenerationConfig,
    #[serde(skip, default)]
    min_size: u32,
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
            output: SpriteSheetState::Empty,
            last_error: None,
            last_editor_paths: Vec::new(),
            view_settings: Default::default(),
            show_modal: false,
        }
    }
}

impl Application {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_config(&mut self, config: rpack_cli::TilemapGenerationConfig) {
        self.data.settings = config;

        let (file_paths, prefix) = self.data.settings.get_file_paths_and_prefix();
        INPUT_QUEUE.push(AppImageAction::Replace(
            file_paths
                .iter()
                .flat_map(|f| f.create_image(&prefix))
                .collect(),
        ));
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
        self.data.min_size = self
            .data
            .image_data
            .iter()
            .max_by(|a, b| a.width.cmp(&b.width))
            .map_or(32, |s| s.width)
            .max(
                self.data
                    .image_data
                    .iter()
                    .max_by(|a, b| a.height.cmp(&b.height))
                    .map_or(32, |s| s.height),
            );
        let config: TexturePackerConfig = (&self.data.settings).into();
        for nr in [32, 64, 128, 256, 512, 1024, 2048, 4096] {
            if nr < self.data.min_size {
                continue;
            }
            let mut packer = SkylinePacker::new(TexturePackerConfig {
                max_width: nr,
                max_height: nr,
                ..config
            });
            let mut success = true;
            for image in &self.data.image_data {
                let data = Rect {
                    x: 0,
                    y: 0,
                    w: image.width,
                    h: image.height,
                };
                if !packer.can_pack(&data) || packer.pack(&data).is_none() {
                    success = false;
                    break;
                }
            }
            if success && nr >= self.data.min_size {
                self.data.min_size = nr;
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

        let mut last_editor_paths: Vec<String> = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };
        let view_settings: ViewSettings = if let Some(storage) = cc.storage {
            eframe::get_value(storage, "view_settings").unwrap_or_default()
        } else {
            Default::default()
        };
        let mut app = Self {
            last_editor_paths,
            view_settings,
            ..Default::default()
        };
        cc.egui_ctx.include_bytes("bytes://image.png", ICON_DATA);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(config_file) = config_file {
            if let Ok(config) = rpack_cli::TilemapGenerationConfig::read_from_file(&config_file) {
                app.read_config(config);
            }
        }

        app
    }

    fn build_atlas(&mut self, ctx: &egui::Context) {
        self.last_error = None;
        ctx.forget_image("bytes://output.png");
        if self.data.image_data.is_empty() {
            self.output = SpriteSheetState::Empty;
            return;
        }
        self.output = SpriteSheetState::Building;
        let mut packer_config: TexturePackerConfig = (&self.data.settings).into();
        if packer_config.max_height < self.data.min_size {
            packer_config.max_height = self.data.min_size;
        }
        if packer_config.max_width < self.data.min_size {
            packer_config.max_width = self.data.min_size;
        }
        let images: Vec<ImageFile> = self
            .data
            .image_data
            .iter()
            .map(|file| file.data.clone())
            .collect();
        let config = SpritesheetBuildConfig {
            packer_config,
            skip_metadata_serialization: self
                .data
                .settings
                .skip_serializing_metadata
                .unwrap_or_default(),
        };
        let path = format!("{}.png", &self.data.settings.output_path);
        execute(async move {
            let result = Spritesheet::build(config, &images, &path);
            INPUT_QUEUE.push(AppImageAction::UpdateSpriteSheet(result));
        });
    }

    fn save_json(&self) -> Result<(), String> {
        let SpriteSheetState::Ok(spritesheet) = &self.output else {
            return Err("Data is incorrect".to_owned());
        };
        let data = spritesheet.atlas_asset_json.to_string();
        let filename = format!("{}.rpack.json", self.data.settings.output_path);
        execute(async move {
            let file_handle = rfd::AsyncFileDialog::new()
                .set_directory(".")
                .add_filter(".rpack.json", &["rpack.json"])
                .set_file_name(filename)
                .save_file()
                .await;

            if let Some(file_handle) = file_handle {
                let _ = file_handle.write(data.as_bytes()).await;
            }
        });
        Ok(())
    }

    fn read_files(&self) {
        let common_prefix = self.get_common_prefix();
        let working_dir = std::path::absolute(common_prefix)
            .map_or(String::from("."), |p| p.to_string_lossy().to_string());
        execute(async move {
            #[cfg(target_arch = "wasm32")]
            let title = "Open Images";
            #[cfg(target_arch = "wasm32")]
            let files = ["png", "jpg", "jpeg", "dds"];
            #[cfg(not(target_arch = "wasm32"))]
            let title = "Open Images or config";
            #[cfg(not(target_arch = "wasm32"))]
            let files = ["png", "jpg", "jpeg", "dds", "json"];
            let file_handles = rfd::AsyncFileDialog::new()
                .set_directory(&working_dir)
                .set_title(title)
                .add_filter("Files", &files)
                .pick_files()
                .await;

            if let Some(file_handles) = file_handles {
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(file) = file_handles
                    .iter()
                    .find(|s| s.file_name().ends_with("rpack_gen.json"))
                {
                    if let Ok(config) =
                        rpack_cli::TilemapGenerationConfig::read_from_file(file.path())
                    {
                        INPUT_QUEUE.push(AppImageAction::ReadFromConfig(
                            config,
                            file.path().to_path_buf(),
                        ));
                    }
                    return;
                }
                for file in file_handles {
                    let content = file.read().await;
                    #[cfg(target_arch = "wasm32")]
                    let name = file.file_name();
                    #[cfg(not(target_arch = "wasm32"))]
                    let name = file.path().to_string_lossy().to_string();
                    if let Some(image) = (content, name).create_image("") {
                        INPUT_QUEUE.push(AppImageAction::Add(image));
                    }
                }
            }
        });
    }

    fn save_atlas(&self) -> Result<(), String> {
        let SpriteSheetState::Ok(spritesheet) = &self.output else {
            return Err("Data is incorrect".to_owned());
        };
        let filename = format!("{}.png", self.data.settings.output_path);
        let mut data = vec![];
        let Ok(()) = spritesheet.image_data.write_to(
            &mut std::io::Cursor::new(&mut data),
            image::ImageFormat::Png,
        ) else {
            return Err("Failed to copy data".to_owned());
        };
        execute(async move {
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
        Ok(())
    }
}

impl eframe::App for Application {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.last_editor_paths);
        eframe::set_value(storage, "view_settings", &self.view_settings);
    }
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        {
            #[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
            puffin::profile_scope!("handle_undo");
            self.undoer
                .feed_state(ctx.input(|input| input.time), &self.data);
        }
        if !INPUT_QUEUE.is_empty() {
            let mut rebuild = false;
            #[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
            puffin::profile_scope!("loading_images");

            #[allow(dead_code)]
            while let Some(cmd) = INPUT_QUEUE.pop() {
                match cmd {
                    AppImageAction::Add(image) => {
                        rebuild = true;
                        let mut out_vec = vec![];
                        image
                            .data
                            .image
                            .thumbnail(64, 64)
                            .write_to(
                                &mut std::io::Cursor::new(&mut out_vec),
                                image::ImageFormat::Png,
                            )
                            .unwrap();
                        ctx.include_bytes(format!("bytes://{}", image.path), out_vec);
                        self.data.image_data.push(image);
                    }
                    AppImageAction::Remove(i) => {
                        rebuild = true;
                        self.data.image_data.remove(i);
                    }
                    AppImageAction::Clear => {
                        rebuild = true;
                        self.data.image_data.clear();
                    }
                    AppImageAction::RebuildAtlas => {
                        rebuild = true;
                        // Will be called after this loop
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    AppImageAction::ReadFromConfig(config, path) => {
                        let path_str = path.to_string_lossy().to_string();
                        if self.last_editor_paths.iter().all(|p| path_str != *p) {
                            self.last_editor_paths.insert(0, path_str);
                            if self.last_editor_paths.len() > 3 {
                                self.last_editor_paths.pop();
                            }
                        }
                        self.read_config(config);
                    }
                    AppImageAction::Replace(new_images) => {
                        rebuild = true;
                        self.data.image_data.clear();
                        self.data.image_data.extend(new_images);
                        for image in &self.data.image_data {
                            let mut out_vec = vec![];
                            image
                                .data
                                .image
                                .thumbnail(64, 64)
                                .write_to(
                                    &mut std::io::Cursor::new(&mut out_vec),
                                    image::ImageFormat::Png,
                                )
                                .unwrap();
                            ctx.include_bytes(format!("bytes://{}", image.path), out_vec);
                        }
                    }
                    AppImageAction::UpdateSpriteSheet(result) => match result {
                        Ok(spritesheet) => {
                            let mut out_vec = vec![];
                            spritesheet
                                .image_data
                                .write_to(
                                    &mut std::io::Cursor::new(&mut out_vec),
                                    image::ImageFormat::Png,
                                )
                                .unwrap();
                            ctx.include_bytes("bytes://output.png", out_vec);
                            self.output = SpriteSheetState::Ok(spritesheet);
                        }
                        Err(e) => {
                            self.last_error = Some(e);
                        }
                    },
                }
            }
            if rebuild {
                self.rebuild_image_data();
                self.build_atlas(ctx);
            }
        }
        if self.show_modal {
            let mut should_close = false;

            should_close |= egui::Modal::new("VisualSettings".into())
                .frame(egui::Frame::menu(&ctx.style()).inner_margin(10.0))
                .show(ctx, |ui| {
                    ui.style_mut().interaction.selectable_labels = false;
                    ui.vertical_centered(|ui| {
                        ui.heading("Settings");
                        ui.add_space(15.0);
                        Grid::new("settings_grid")
                            .num_columns(2)
                            .striped(true)
                            .show(ui, |ui| {
                                ui.add(Label::new("Max Preview size"));
                                ui.add(Slider::new(
                                    &mut self.view_settings.preview_max_size,
                                    256.0..=1024.0,
                                ));
                                ui.end_row();
                                ui.add(Label::new("Display JSON"));
                                ui.add(Checkbox::new(&mut self.view_settings.display_json, ""));
                                ui.end_row();
                            });
                        ui.add_space(10.0);
                        should_close |= ui.button("Close").clicked();
                    });
                })
                .should_close();
            if should_close {
                self.show_modal = false;
            }
        }
        egui::TopBottomPanel::top("topPanel")
            .frame(egui::Frame::canvas(&ctx.style()))
            .show(ctx, |ui| {
                #[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
                puffin::profile_scope!("top_panel");
                // ui.add_space(TOP_SIDE_MARGIN);
                #[cfg(not(target_arch = "wasm32"))]
                let title_font = FontId::new(26.0, FontFamily::Name("semibold".into()));
                #[cfg(target_arch = "wasm32")]
                let title_font = FontId::new(26.0, FontFamily::Proportional);
                ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    ui.add(
                        Label::new(
                            egui::RichText::new("rPack")
                                .font(title_font)
                                .color(MY_ACCENT_COLOR32)
                                .strong(),
                        )
                        .selectable(false),
                    );
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(10.0);
                        if self.output.is_none() && ui.add(egui::Button::new("Open")).clicked() {
                            self.read_files();
                        }
                        if self.output.is_ok() {
                            if ui.add(egui::Button::new("Save atlas image")).clicked() {
                                if let Err(error) = self.save_atlas() {
                                    eprintln!("ERROR: {}", error);
                                }
                            }
                            if ui.add(egui::Button::new("Save atlas json")).clicked() {
                                if let Err(error) = self.save_json() {
                                    eprintln!("ERROR: {}", error);
                                }
                            }
                        }
                        if ui.available_width() > 15.0 {
                            ui.add_space(ui.available_width() - 10.0);
                        }
                    });
                });
                // ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                //     let text = ;
                //     ui.allocate_space(egui::vec2(TOP_SIDE_MARGIN, HEADER_HEIGHT));
                //     ui.add(egui::Label::new(text).selectable(false));
                // });
            });
        ctx.input(|i| {
            #[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
            puffin::profile_scope!("dropped_files");
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
                        for entry in dir.flatten() {
                            if let Some(dyn_image) = entry.metadata().ok().and_then(|metadata| {
                                if metadata.is_file() {
                                    entry.create_image("")
                                } else {
                                    None
                                }
                            }) {
                                INPUT_QUEUE.push(AppImageAction::Add(dyn_image));
                            }
                        }
                    } else {
                        let Some(path) = &file.path else {
                            continue;
                        };
                        if path.to_string_lossy().ends_with(".rpack_gen.json") {
                            if let Ok(config) =
                                rpack_cli::TilemapGenerationConfig::read_from_file(path)
                            {
                                INPUT_QUEUE
                                    .push(AppImageAction::ReadFromConfig(config, path.clone()));
                                break;
                            }
                        }
                    }
                }
                if let Some(dyn_image) = file.create_image("") {
                    INPUT_QUEUE.push(AppImageAction::Add(dyn_image));
                }
            }
        });
        egui::TopBottomPanel::bottom("bottom_panel")
            .frame(egui::Frame::canvas(&ctx.style()))
            .show(ctx, |ui| {
                #[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
                puffin::profile_scope!("bottom_panel");
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.add_space(5.0);
                    if ui.button("🛠").on_hover_text("Visual settings").clicked() {
                        self.show_modal = true;
                    }
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
            .frame(egui::Frame::canvas(&ctx.style()).inner_margin(10))
            .show_animated(ctx, !self.data.image_data.is_empty(), |ui| {
                ui.with_layout(
                    Layout::top_down(egui::Align::Min).with_cross_justify(true),
                    |ui| {
                        ui.style_mut().interaction.selectable_labels = false;
                        {
                            #[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
                            puffin::profile_scope!("right_panel");
                            let mut changed = false;
                            Grid::new("settings_grid")
                                .num_columns(2)
                                .spacing((0.0, 10.0))
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.style_mut().visuals.faint_bg_color =
                                        Color32::from_white_alpha(15);
                                    ui.style_mut().interaction.selectable_labels = false;
                                    let id = ui.label("File Name").id;
                                    if ui
                                        .text_edit_singleline(&mut self.data.settings.output_path)
                                        .labelled_by(id)
                                        .changed()
                                    {
                                        if let SpriteSheetState::Ok(data) = &mut self.output {
                                            data.atlas_asset.filename =
                                                self.data.settings.output_path.clone();
                                            data.rebuild_json();
                                        }
                                    }
                                    ui.end_row();
                                    ui.label("Output size");
                                    let selected_size = match self.data.settings.size {
                                        Some(size) => {
                                            format!("{}x{}", size, size)
                                        }
                                        None => String::from("2048x2048"),
                                    };
                                    changed |= MenuButton::from_button(
                                        Button::new(selected_size).frame_when_inactive(false),
                                    )
                                    .ui(ui, |ui| {
                                        for size in [32, 64, 128, 256, 512, 1024, 2048, 4096] {
                                            let same_size =
                                                self.data.settings.size.is_some_and(|s| s == size);
                                            if ui
                                                .add_enabled(
                                                    size >= self.data.min_size,
                                                    Button::new(format!("{}x{}", size, size))
                                                        .selected(same_size),
                                                )
                                                .clicked()
                                            {
                                                self.data.settings.size =
                                                    if same_size { None } else { Some(size) };
                                                return true;
                                            }
                                        }
                                        false
                                    })
                                    .1
                                    .is_some_and(|s| s.inner);
                                    ui.end_row();
                                    changed |= slider_field(
                                        ui,
                                        "Border Padding",
                                        &mut self.data.settings.border_padding,
                                        0,
                                        0..=10,
                                    );
                                    ui.end_row();
                                    changed |= slider_field(
                                        ui,
                                        "Texture Padding",
                                        &mut self.data.settings.texture_padding,
                                        2,
                                        0..=10,
                                    );
                                    ui.end_row();
                                    let mut skip_metadata = self
                                        .data
                                        .settings
                                        .skip_serializing_metadata
                                        .unwrap_or_default();
                                    ui.label("Skip Metadata Serialization");
                                    if ui.checkbox(&mut skip_metadata, "").changed() {
                                        self.data.settings.skip_serializing_metadata =
                                            Some(skip_metadata);
                                        if let SpriteSheetState::Ok(data) = &mut self.output {
                                            data.atlas_asset.metadata.skip_serialization =
                                                skip_metadata;
                                            data.rebuild_json();
                                        }
                                    }
                                });
                            if changed {
                                INPUT_QUEUE.push(AppImageAction::RebuildAtlas);
                            }
                        }
                        Grid::new("ImgesHeader")
                            .num_columns(2)
                            .spacing([50.0, 10.0])
                            .show(ui, |ui| {
                                ui.menu_button("🖼", |ui| {
                                    if !self.data.image_data.is_empty()
                                        && ui
                                            .add(
                                                egui::Button::new("Remove all images").frame(false),
                                            )
                                            .clicked()
                                    {
                                        INPUT_QUEUE.push(AppImageAction::Clear);
                                    }
                                    ui.add_space(10.0);

                                    if ui
                                        .add(egui::Button::new("Add more images").frame(false))
                                        .clicked()
                                    {
                                        self.read_files();
                                    }
                                });
                                ui.heading("Images");
                            });

                        ui.add_enabled_ui(!self.output.is_building(), |ui| {
                            ui.separator();
                            {
                                #[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
                                puffin::profile_scope!("image_list");

                                let length = self.data.image_data.len();
                                let table = TableBuilder::new(ui)
                                    .striped(true)
                                    .vscroll(true)
                                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                    .column(Column::exact(70.0))
                                    .column(Column::remainder().at_least(50.0))
                                    .sense(egui::Sense::click());
                                table.body(|body| {
                                    body.rows(64.0, length, |mut row| {
                                        let index = row.index();
                                        let file = &self.data.image_data[index];
                                        row.col(|ui| {
                                            ui.vertical_centered_justified(|ui| {
                                                ui.add_sized(
                                                    (64.0, 64.0),
                                                    Image::from_uri(format!(
                                                        "bytes://{}",
                                                        file.path.as_str()
                                                    ))
                                                    .corner_radius(5u8),
                                                )
                                                .on_hover_text(format!(
                                                    "{}x{}",
                                                    file.width, file.height
                                                ));
                                            });
                                        });
                                        row.col(|ui| {
                                            ui.add(
                                                Label::new(file.id())
                                                    .selectable(false)
                                                    .wrap_mode(egui::TextWrapMode::Truncate),
                                            );
                                        })
                                        .1
                                        .context_menu(
                                            |ui| {
                                                if ui
                                                    .add(Button::new("Remove").frame(false))
                                                    .clicked()
                                                {
                                                    INPUT_QUEUE.push(AppImageAction::Remove(index));
                                                }
                                            },
                                        );
                                    });
                                });
                            }
                        });
                    },
                );
            });
        egui::CentralPanel::default()
            .frame(Frame::central_panel(&ctx.style()).inner_margin(16i8))
            .show(ctx, |ui| {
                #[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
                puffin::profile_scope!("central_panel");
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
                        ui.vertical_centered_justified(|ui| {
                            if self.data.image_data.is_empty() {
                                ui.add_space(25.0);
                                Grid::new("Header").num_columns(2).show(ui, |ui|{
                                   ui.add_sized(
                                       (256.0,256.0),Image::new("bytes://image.png"));
                                   ui.vertical_centered_justified(|ui|{
                                       ui.add_space(15.0);
                                       ui.add(
                                           Label::new(
                                               RichText::new("Create spritesheets in seconds!")
                                                   .heading()
                                                   // .color(MY_ACCENT_COLOR32),
                                           )
                                           .selectable(false),
                                       );
                                       ui.separator();
                                       ui.add_space(10.0);
                                       ui.add(
                                           Label::new(
                                               RichText::new("Drag and drop images here to start creating new spritesheets.")
                                           )
                                           .selectable(false),
                                       );
                                   });
                                });
                                ui.add_space(10.0);
                                // if ui.button("Open files").clicked() {
                                //     self.read_files();
                                // }
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    if !self.last_editor_paths.is_empty() {
                                        ui.separator();
                                        ui.add(
                                            Label::new(RichText::new("Recent projects").heading())
                                                .selectable(false),
                                        );
                                    }
                                    ui.add_space(10.0);
                                    for p in &self.last_editor_paths {
                                        if ui.add(Button::new(p).frame(false)).clicked() {
                                            if let Ok(config) =
                                                rpack_cli::TilemapGenerationConfig::read_from_file(
                                                    p,
                                                )
                                            {
                                                use std::str::FromStr;

                                                INPUT_QUEUE.push(AppImageAction::ReadFromConfig(
                                                    config,
                                                    PathBuf::from_str(p).unwrap_or_default(),
                                                ));
                                            }
                                        }
                                        ui.add_space(10.0);
                                    }
                                }
                            }
                            if self.output.is_building(){
                                ui.heading(
                                    egui::RichText::new("Building atlas...").color(MY_ACCENT_COLOR32),
                                );
                                ui.add_space(10.0);
                                ui.spinner();
                            }
                            let SpriteSheetState::Ok(data) = &self.output else {
                                return;
                            };
                            ui.add(Image::from_uri("bytes://output.png").bg_fill(Color32::from_black_alpha(200)).max_size(Vec2::splat(self.view_settings.preview_max_size))).on_hover_text(format!(
                                "{} sprites\nsize: {}x{}",
                                data.atlas_asset.frames.len(),
                                data.atlas_asset.size[0],
                                data.atlas_asset.size[1]
                            ));
                            ui.separator();
                            ui.add_space(10.0);
                            ui.horizontal(|ui|{
                                if self.view_settings.display_json {
                                ui.vertical_centered_justified(|ui|{
                                    ui.add_space(10.0);
                                    if ui
                                        .add(egui::Button::new("Copy JSON to Clipboard"))
                                        .clicked()
                                    {
                                        ui.ctx()
                                            .copy_text(data.atlas_asset_json.to_string());
                                    };
                                        ui.add_space(10.0);
                                        egui_json_tree::JsonTree::new(
                                            "simple-tree",
                                            &data.atlas_asset_json,
                                        )
                                        .show(ui);
                                });
                                }

                            });

                            ui.add_space(20.0);
                        });
                    });
            });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.with_layout(Layout::right_to_left(egui::Align::Min), |ui| {
        ui.add_space(10.0);
        if ui
            .add(egui::Button::new(VERSION).frame(false))
            .on_hover_text(GIT_HASH)
            .clicked()
        {
            ui.ctx()
                .open_url(egui::OpenUrl::new_tab(env!("CARGO_PKG_HOMEPAGE")));
        }
        ui.separator();
        egui::widgets::global_theme_preference_switch(ui);
        ui.separator();
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.hyperlink_to("Mev Lyshkin", "https://www.mevlyshkin.com/");
        ui.add_space(10.0);
        ui.label("Made by");
        ui.add_space((ui.available_width() - 10.0).max(15.0));
    });
}

fn slider_field(
    ui: &mut Ui,
    label: &str,
    field: &mut Option<u32>,
    default_value: u32,
    range: RangeInclusive<u32>,
) -> bool {
    ui.label(label);
    let mut value = field.unwrap_or(default_value);
    if ui.add(Slider::new(&mut value, range)).changed() {
        *field = Some(value);
        true
    } else {
        false
    }
}
