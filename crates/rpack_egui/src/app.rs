use crate::helpers::DroppedFileHelper;
#[cfg(not(target_arch = "wasm32"))]
use crate::helpers::IMAGE_EXTENSIONS;
use crate::notifications::{Level, Notifications};
use crate::preview::{self, PreviewState};
use crate::view_settings::ViewSettings;
use crossbeam::queue::SegQueue;
use egui::containers::menu::MenuButton;
use egui::{
    Button, Color32, Context, FontFamily, FontId, Frame, Grid, Image, Key, KeyboardShortcut, Label,
    Layout, Modifiers, RichText, Slider, Ui, util::undoer::Undoer,
};
use egui_extras::{Column, TableBuilder};
use egui_material_icons::{MaterialIcon, icons::*};
use once_cell::sync::Lazy;
use rpack_cli::TilemapGenerationConfig;
use rpack_cli::{ImageFile, Spritesheet, SpritesheetBuildConfig, SpritesheetError};
use std::collections::HashSet;
use std::sync::Arc;
use std::{future::Future, ops::RangeInclusive, time::Duration};
use texture_packer::TexturePackerConfig;

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

static INPUT_QUEUE: Lazy<SegQueue<AppImageAction>> = Lazy::new(SegQueue::new);
static REPAINT_CTX: std::sync::OnceLock<egui::Context> = std::sync::OnceLock::new();
/// Id of the most recently started atlas build; results of older builds are discarded.
static BUILD_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub const MY_ACCENT_COLOR32: Color32 = Color32::from_rgb(230, 102, 1);
/// Darker variant of the accent used behind white text (selection, active sliders).
const ACCENT_FILL: Color32 = Color32::from_rgb(188, 78, 0);
pub const GIT_HASH: &str = env!("GIT_HASH");
pub const VERSION: &str = concat!(" v ", env!("CARGO_PKG_VERSION"), " ");
pub const ICON_DATA: &[u8] = include_bytes!("../static/base_icon.png");

const DEFAULT_ATLAS_SIZE: u32 = 2048;
const ATLAS_SIZES: [u32; 8] = [32, 64, 128, 256, 512, 1024, 2048, 4096];
const REBUILD_DEBOUNCE: f64 = 0.15;
#[cfg(not(target_arch = "wasm32"))]
const MAX_RECENT_PROJECTS: usize = 8;
const DEFAULT_OUTPUT_NAME: &str = "atlas";

const OPEN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
const SAVE_ATLAS: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
const SAVE_JSON: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::S);
const UNDO: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Z);
const REDO: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Z);
const REDO_ALT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Y);
const SELECT_ALL: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::A);
const REMOVE_SELECTED: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);
const REMOVE_SELECTED_ALT: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::NONE, Key::Backspace);

#[derive(Clone)]
pub enum AppImageAction {
    Add(Vec<AppImageData>),
    Replace(Vec<AppImageData>),
    Remove(Vec<String>),
    UpdateSpriteSheet(u64, BuildOutcome),
    Clear,
    RebuildAtlas,
    Notify(Level, String),
    #[cfg(not(target_arch = "wasm32"))]
    ReadFromConfig(TilemapGenerationConfig, PathBuf),
    #[cfg(not(target_arch = "wasm32"))]
    AddRecent(String),
}

/// Queues a command and wakes the UI so it is handled without waiting for user input.
fn push_cmd(cmd: AppImageAction) {
    INPUT_QUEUE.push(cmd);
    if let Some(ctx) = REPAINT_CTX.get() {
        ctx.request_repaint();
    }
}

fn notify(level: Level, text: impl Into<String>) {
    push_cmd(AppImageAction::Notify(level, text.into()));
}

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    // this is stupid... use any executor of your choice instead
    std::thread::spawn(move || futures::executor::block_on(f));
}
#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

macro_rules! profile_scope {
    ($name:expr) => {
        #[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
        puffin::profile_scope!($name);
    };
}

/// A finished atlas together with its PNG encoding, which is used for the preview and saving.
#[derive(Clone)]
pub struct Built {
    sheet: Spritesheet,
    png: Arc<[u8]>,
}

impl std::ops::Deref for Built {
    type Target = Spritesheet;
    fn deref(&self) -> &Spritesheet {
        &self.sheet
    }
}

impl std::ops::DerefMut for Built {
    fn deref_mut(&mut self) -> &mut Spritesheet {
        &mut self.sheet
    }
}

#[derive(Clone)]
pub struct BuildOutcome {
    /// Smallest square atlas side that fits all images, `0` if even the largest does not.
    min_size: u32,
    result: Result<Built, SpritesheetError>,
}

#[allow(clippy::large_enum_variant)]
pub enum SpriteSheetState {
    Empty,
    Ok(Built),
}
impl SpriteSheetState {
    pub fn is_ok(&self) -> bool {
        matches!(self, SpriteSheetState::Ok(_))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Open,
    SaveAtlas,
    SaveJson,
    #[cfg(not(target_arch = "wasm32"))]
    SaveConfig,
    Undo,
    Redo,
    SelectAll,
    RemoveSelected,
    Clear,
    FitPreview,
}

pub struct Application {
    data: ApplicationData,
    /// Last successfully built atlas. It stays visible (dimmed) while a new one is building.
    output: SpriteSheetState,
    building: bool,
    min_size: u32,
    last_error: Option<SpritesheetError>,
    undoer: Undoer<ApplicationData>,
    last_editor_paths: Vec<String>,
    view_settings: ViewSettings,
    applied_scale: f32,
    json_preview: crate::json_preview::JsonPreview,
    notifications: Notifications,
    preview: PreviewState,
    /// Paths of the selected images.
    selected: HashSet<String>,
    selection_anchor: Option<usize>,
    /// Id of the sprite under the cursor in the atlas preview, highlighted in the image list.
    hovered_frame: Option<String>,
    /// Last hovered sprite the image list was scrolled to.
    scrolled_to: Option<String>,
    /// Time after which a debounced rebuild is started.
    pending_rebuild: Option<f64>,
    window_title: String,
}

/// State covered by undo/redo.
#[derive(Default, Clone, PartialEq)]
pub struct ApplicationData {
    image_data: Vec<AppImageData>,
    settings: TilemapGenerationConfig,
}

#[derive(Clone)]
pub struct AppImageData {
    pub width: u32,
    pub height: u32,
    pub id: String,
    pub image: Arc<image::DynamicImage>,
    pub path: String,
    thumbnail: Arc<[u8]>,
}

// Pixel data is intentionally not compared: it is expensive and a path identifies an image.
impl PartialEq for AppImageData {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
            && self.id == other.id
            && self.width == other.width
            && self.height == other.height
    }
}

impl AppImageData {
    pub fn new(path: String, id: String, image: image::DynamicImage) -> Self {
        let mut thumbnail = vec![];
        let _ = image.thumbnail(64, 64).write_to(
            &mut std::io::Cursor::new(&mut thumbnail),
            image::ImageFormat::Png,
        );
        Self {
            width: image.width(),
            height: image.height(),
            id,
            image: Arc::new(image),
            path,
            thumbnail: thumbnail.into(),
        }
    }

    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    pub fn update_id(&mut self, prefix: &str) {
        let path = self.path.replace('\\', "/");
        self.id = rpack_cli::packing::frame_key(path.strip_prefix(prefix).unwrap_or(&path));
    }

    fn thumbnail_uri(&self) -> String {
        format!("bytes://{}", self.path)
    }
}

impl Default for Application {
    fn default() -> Self {
        let settings = TilemapGenerationConfig {
            output_path: DEFAULT_OUTPUT_NAME.to_owned(),
            ..Default::default()
        };
        Self {
            data: ApplicationData {
                settings,
                ..Default::default()
            },
            undoer: Default::default(),
            output: SpriteSheetState::Empty,
            building: false,
            min_size: 0,
            last_error: None,
            last_editor_paths: Vec::new(),
            view_settings: Default::default(),
            applied_scale: 0.0,
            json_preview: Default::default(),
            notifications: Default::default(),
            preview: Default::default(),
            selected: HashSet::new(),
            selection_anchor: None,
            hovered_frame: None,
            scrolled_to: None,
            pending_rebuild: None,
            window_title: String::new(),
        }
    }
}

/// Finds the smallest square atlas side that fits the images.
/// Returns `0` and the last packing error when none of the supported sizes does.
fn find_min_size(
    config: TexturePackerConfig,
    dimensions: &[(&str, u32, u32)],
) -> (u32, Option<SpritesheetError>) {
    if dimensions.is_empty() {
        return (ATLAS_SIZES[0], None);
    }
    let mut last_error = None;
    for size in ATLAS_SIZES {
        match rpack_cli::packing::place(
            TexturePackerConfig {
                max_width: size,
                max_height: size,
                ..config
            },
            dimensions,
        ) {
            Ok(_) => return (size, None),
            Err(error) => last_error = Some(error.into()),
        }
    }
    (0, last_error)
}

struct BuildInput {
    id: String,
    image: Arc<image::DynamicImage>,
    width: u32,
    height: u32,
}

/// Runs on a worker thread: finds the minimum size, packs and encodes the preview.
fn run_build(settings: TilemapGenerationConfig, inputs: Vec<BuildInput>) -> BuildOutcome {
    let config: TexturePackerConfig = (&settings).into();
    let dimensions: Vec<_> = inputs
        .iter()
        .map(|input| (input.id.as_str(), input.width, input.height))
        .collect();
    let (min_size, error) = find_min_size(config, &dimensions);
    if min_size == 0 {
        return BuildOutcome {
            min_size,
            result: Err(error.unwrap_or(SpritesheetError::FailedToPackImage)),
        };
    }
    let mut packer_config = config;
    packer_config.max_width = packer_config.max_width.max(min_size);
    packer_config.max_height = packer_config.max_height.max(min_size);
    let files: Vec<ImageFile> = inputs
        .into_iter()
        .map(|input| ImageFile {
            id: input.id,
            image: (*input.image).clone(),
        })
        .collect();
    let build_config = SpritesheetBuildConfig {
        packer_config,
        skip_metadata_serialization: settings.skip_serializing_metadata.unwrap_or_default(),
    };
    let filename = format!("{}.png", settings.output_path);
    let result = Spritesheet::build(build_config, &files, &filename).and_then(|sheet| {
        let mut png = vec![];
        sheet
            .image_data
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|_| SpritesheetError::FailedToExportImage)?;
        Ok(Built {
            sheet,
            png: png.into(),
        })
    });
    BuildOutcome { min_size, result }
}

/// Decodes image files and queues them. Runs on a worker thread.
#[cfg(not(target_arch = "wasm32"))]
fn load_paths(paths: Vec<PathBuf>, replace: bool) {
    let mut images = Vec::new();
    let mut failed = Vec::new();
    for path in paths {
        match path.create_image("") {
            Some(image) => images.push(image),
            None => failed.push(path.display().to_string()),
        }
    }
    report_loaded(images, failed, replace);
}

fn report_loaded(images: Vec<AppImageData>, failed: Vec<String>, replace: bool) {
    if !failed.is_empty() {
        let names: Vec<_> = failed.iter().take(3).map(|name| short_name(name)).collect();
        let more = failed.len().saturating_sub(names.len());
        let mut text = format!("Could not read {}", names.join(", "));
        if more > 0 {
            text.push_str(&format!(" and {more} more"));
        }
        notify(Level::Error, text);
    }
    if replace {
        push_cmd(AppImageAction::Replace(images));
    } else if !images.is_empty() {
        push_cmd(AppImageAction::Add(images));
    }
}

fn short_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

#[cfg(not(target_arch = "wasm32"))]
fn has_image_extension(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| IMAGE_EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// Expands dropped directories (one level deep) into the image files they contain.
#[cfg(not(target_arch = "wasm32"))]
fn expand_dropped(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            let Ok(dir) = path.read_dir() else {
                continue;
            };
            let mut entries: Vec<_> = dir
                .flatten()
                .map(|entry| entry.path())
                .filter(|p| p.is_file() && has_image_extension(p))
                .collect();
            entries.sort();
            files.extend(entries);
        } else {
            files.push(path);
        }
    }
    files
}

#[cfg(not(target_arch = "wasm32"))]
fn glob_escape(path: &str) -> String {
    path.replace('\\', "/")
        .replace('[', "[[]")
        .replace('*', "[*]")
        .replace('?', "[?]")
}

fn apply_style(ctx: &Context, scale: f32) {
    let defaults = egui::Style::default().text_styles;
    ctx.all_styles_mut(|style| {
        for (text_style, font_id) in style.text_styles.iter_mut() {
            if let Some(base) = defaults.get(text_style) {
                font_id.size = base.size * scale;
            }
        }
        style.visuals.selection.bg_fill = ACCENT_FILL;
        style.visuals.selection.stroke.color = Color32::WHITE;
    });
}

fn icon_label(icon: MaterialIcon, text: &str) -> String {
    format!("{}  {}", icon.codepoint, text)
}

/// A menu entry with an optional shortcut hint. Closes the menu when clicked.
fn menu_item(
    ui: &mut Ui,
    icon: MaterialIcon,
    text: &str,
    shortcut: Option<&KeyboardShortcut>,
    enabled: bool,
) -> bool {
    let mut button = Button::new(icon_label(icon, text));
    if let Some(shortcut) = shortcut {
        button = button.shortcut_text(ui.ctx().format_shortcut(shortcut));
    }
    let clicked = ui.add_enabled(enabled, button).clicked();
    if clicked {
        ui.close();
    }
    clicked
}

fn hint(ctx: &Context, text: &str, shortcut: &KeyboardShortcut) -> String {
    format!("{text} ({})", ctx.format_shortcut(shortcut))
}

impl Application {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>, config_file: Option<String>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let _ = REPAINT_CTX.set(cc.egui_ctx.clone());

        #[allow(unused_mut)]
        let mut last_editor_paths: Vec<String> = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };
        #[cfg(not(target_arch = "wasm32"))]
        last_editor_paths.retain(|path| std::path::Path::new(path).exists());
        let view_settings: ViewSettings = if let Some(storage) = cc.storage {
            eframe::get_value(storage, "view_settings").unwrap_or_default()
        } else {
            Default::default()
        };
        #[cfg(not(target_arch = "wasm32"))]
        crate::fonts::load_fonts(&cc.egui_ctx);
        #[cfg(target_arch = "wasm32")]
        crate::fonts::register_semibold_fallback(&cc.egui_ctx);
        // Must come after the fonts are set, otherwise the icon font would be replaced.
        egui_material_icons::initialize(&cc.egui_ctx);
        apply_style(&cc.egui_ctx, view_settings.ui_scale);
        cc.egui_ctx.include_bytes("bytes://image.png", ICON_DATA);

        #[allow(unused_mut)]
        let mut app = Self {
            last_editor_paths,
            applied_scale: view_settings.ui_scale,
            view_settings,
            ..Default::default()
        };
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(config_file) = config_file {
            match rpack_cli::TilemapGenerationConfig::read_from_file(&config_file) {
                Ok(config) => app.read_config(config),
                Err(error) => notify(Level::Error, format!("Could not read config: {error}")),
            }
        }
        #[cfg(target_arch = "wasm32")]
        let _ = config_file;

        app
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_config(&mut self, config: rpack_cli::TilemapGenerationConfig) {
        self.data.settings = config.clone();
        execute(async move {
            let (file_paths, _) = config.get_file_paths_and_prefix();
            if file_paths.is_empty() {
                notify(
                    Level::Error,
                    "No files matched the asset patterns of the config",
                );
                return;
            }
            load_paths(file_paths, true);
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn add_recent(&mut self, path: String) {
        self.last_editor_paths.retain(|p| *p != path);
        self.last_editor_paths.insert(0, path);
        self.last_editor_paths.truncate(MAX_RECENT_PROJECTS);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_recent(&mut self, path: &str) {
        match rpack_cli::TilemapGenerationConfig::read_from_file(path) {
            Ok(config) => push_cmd(AppImageAction::ReadFromConfig(config, PathBuf::from(path))),
            Err(error) => {
                self.last_editor_paths.retain(|p| p != path);
                notify(Level::Error, format!("Could not open {path}: {error}"));
            }
        }
    }

    pub fn get_common_prefix(&self) -> String {
        let file_paths: Vec<&str> = self
            .data
            .image_data
            .iter()
            .map(|image| image.path.as_str())
            .collect();
        rpack_cli::get_common_prefix(&file_paths)
    }

    /// Recomputes frame ids and returns how many of them are not unique.
    pub fn rebuild_image_data(&mut self) -> usize {
        let prefix = self.get_common_prefix();
        self.data
            .image_data
            .iter_mut()
            .for_each(|f| f.update_id(prefix.as_str()));
        let mut seen = HashSet::new();
        self.data
            .image_data
            .iter()
            .filter(|image| !seen.insert(image.id()))
            .count()
    }

    fn build_atlas(&mut self) {
        self.json_preview.invalidate();
        let generation = BUILD_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        if self.data.image_data.is_empty() {
            self.output = SpriteSheetState::Empty;
            self.building = false;
            self.last_error = None;
            self.min_size = ATLAS_SIZES[0];
            return;
        }
        self.building = true;
        let settings = self.data.settings.clone();
        let inputs: Vec<BuildInput> = self
            .data
            .image_data
            .iter()
            .map(|file| BuildInput {
                id: file.id.clone(),
                image: file.image.clone(),
                width: file.width,
                height: file.height,
            })
            .collect();
        execute(async move {
            let outcome = run_build(settings, inputs);
            push_cmd(AppImageAction::UpdateSpriteSheet(generation, outcome));
        });
    }

    fn save_json(&self) {
        let SpriteSheetState::Ok(built) = &self.output else {
            return;
        };
        let data = serde_json::to_string_pretty(&built.atlas_asset_json)
            .unwrap_or_else(|_| built.atlas_asset_json.to_string());
        let filename = format!("{}.rpack.json", self.data.settings.output_path);
        execute(async move {
            let Some(file) = rfd::AsyncFileDialog::new()
                .set_directory(".")
                .add_filter(".rpack.json", &["rpack.json"])
                .set_file_name(filename)
                .save_file()
                .await
            else {
                return;
            };
            match file.write(data.as_bytes()).await {
                Ok(()) => notify(Level::Info, format!("Saved {}", file.file_name())),
                Err(error) => notify(Level::Error, format!("Failed to save atlas json: {error}")),
            }
        });
    }

    fn save_atlas(&self) {
        let SpriteSheetState::Ok(built) = &self.output else {
            return;
        };
        let png = built.png.clone();
        let filename = format!("{}.png", self.data.settings.output_path);
        execute(async move {
            let Some(file) = rfd::AsyncFileDialog::new()
                .set_directory(".")
                .set_file_name(filename)
                .save_file()
                .await
            else {
                return;
            };
            match file.write(&png).await {
                Ok(()) => notify(Level::Info, format!("Saved {}", file.file_name())),
                Err(error) => notify(Level::Error, format!("Failed to save atlas image: {error}")),
            }
        });
    }

    /// Saves the current inputs and settings as a `.rpack_gen.json` config.
    #[cfg(not(target_arch = "wasm32"))]
    fn save_config(&self) {
        let mut config = self.data.settings.clone();
        let paths: Vec<String> = self
            .data
            .image_data
            .iter()
            .map(|image| image.path.clone())
            .collect();
        let filename = format!("{}.rpack_gen.json", config.output_path);
        execute(async move {
            let Some(file) = rfd::AsyncFileDialog::new()
                .set_directory(".")
                .add_filter("rpack config", &["json"])
                .set_file_name(filename)
                .save_file()
                .await
            else {
                return;
            };
            let dir = file.path().parent().map(|p| p.to_path_buf());
            config.asset_patterns = paths
                .iter()
                .map(|path| {
                    let relative = dir
                        .as_ref()
                        .and_then(|dir| std::path::Path::new(path).strip_prefix(dir).ok())
                        .map(|p| p.to_string_lossy().to_string());
                    glob_escape(relative.as_deref().unwrap_or(path))
                })
                .collect();
            let json = match serde_json::to_string_pretty(&config) {
                Ok(json) => json,
                Err(error) => {
                    notify(Level::Error, format!("Failed to save config: {error}"));
                    return;
                }
            };
            match file.write(json.as_bytes()).await {
                Ok(()) => {
                    notify(Level::Info, format!("Saved {}", file.file_name()));
                    push_cmd(AppImageAction::AddRecent(
                        file.path().to_string_lossy().to_string(),
                    ));
                }
                Err(error) => notify(Level::Error, format!("Failed to save config: {error}")),
            }
        });
    }

    fn read_files(&self) {
        let common_prefix = self.get_common_prefix();
        let working_dir = std::path::absolute(common_prefix)
            .map_or(String::from("."), |p| p.to_string_lossy().to_string());
        execute(async move {
            #[cfg(target_arch = "wasm32")]
            let title = "Add images";
            #[cfg(target_arch = "wasm32")]
            let files = ["png", "jpg", "jpeg", "dds"];
            #[cfg(not(target_arch = "wasm32"))]
            let title = "Add images or open config";
            #[cfg(not(target_arch = "wasm32"))]
            let files = ["png", "jpg", "jpeg", "dds", "json"];
            let Some(file_handles) = rfd::AsyncFileDialog::new()
                .set_directory(&working_dir)
                .set_title(title)
                .add_filter("Files", &files)
                .pick_files()
                .await
            else {
                return;
            };
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(file) = file_handles
                .iter()
                .find(|s| s.file_name().ends_with("rpack_gen.json"))
            {
                match rpack_cli::TilemapGenerationConfig::read_from_file(file.path()) {
                    Ok(config) => {
                        if file_handles.len() > 1 {
                            notify(
                                Level::Warning,
                                "A config was selected, the other files were ignored",
                            );
                        }
                        push_cmd(AppImageAction::ReadFromConfig(
                            config,
                            file.path().to_path_buf(),
                        ));
                    }
                    Err(error) => notify(Level::Error, format!("Could not read config: {error}")),
                }
                return;
            }
            let mut images = Vec::new();
            let mut failed = Vec::new();
            for file in file_handles {
                let content = file.read().await;
                #[cfg(target_arch = "wasm32")]
                let name = file.file_name();
                #[cfg(not(target_arch = "wasm32"))]
                let name = file.path().to_string_lossy().to_string();
                match (content, name.clone()).create_image("") {
                    Some(image) => images.push(image),
                    None => failed.push(name),
                }
            }
            report_loaded(images, failed, false);
        });
    }

    fn handle_dropped_files(&self, ctx: &Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if dropped.is_empty() {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut paths = Vec::new();
            for file in &dropped {
                let path = file.path();
                if path.to_string_lossy().ends_with(".rpack_gen.json") {
                    match rpack_cli::TilemapGenerationConfig::read_from_file(path) {
                        Ok(config) => {
                            push_cmd(AppImageAction::ReadFromConfig(config, path.to_path_buf()))
                        }
                        Err(error) => {
                            notify(Level::Error, format!("Could not read config: {error}"))
                        }
                    }
                    return;
                }
                paths.push(path.to_path_buf());
            }
            execute(async move { load_paths(expand_dropped(paths), false) });
        }
        #[cfg(target_arch = "wasm32")]
        for file in dropped {
            let name = file.path().to_string_lossy().to_string();
            execute(async move {
                let image = match file.bytes_async().await {
                    Ok(bytes) => (bytes, name.clone()).create_image(""),
                    Err(_) => None,
                };
                match image {
                    Some(image) => report_loaded(vec![image], vec![], false),
                    None => report_loaded(vec![], vec![name], false),
                }
            });
        }
    }

    fn drain_commands(&mut self, ctx: &Context) {
        profile_scope!("loading_images");
        let mut rebuild = false;
        let mut check_names = false;
        let mut duplicates = 0;
        while let Some(cmd) = INPUT_QUEUE.pop() {
            match cmd {
                AppImageAction::Add(images) => {
                    for image in images {
                        if self.data.image_data.iter().any(|i| i.path == image.path) {
                            duplicates += 1;
                            continue;
                        }
                        ctx.include_bytes(image.thumbnail_uri(), image.thumbnail.clone());
                        self.data.image_data.push(image);
                        rebuild = true;
                        check_names = true;
                    }
                }
                AppImageAction::Remove(paths) => {
                    rebuild = true;
                    self.data
                        .image_data
                        .retain(|image| !paths.contains(&image.path));
                }
                AppImageAction::Clear => {
                    rebuild = true;
                    self.data.image_data.clear();
                }
                AppImageAction::RebuildAtlas => rebuild = true,
                AppImageAction::Notify(level, text) => self.notifications.push(ctx, level, text),
                #[cfg(not(target_arch = "wasm32"))]
                AppImageAction::ReadFromConfig(config, path) => {
                    self.add_recent(path.to_string_lossy().to_string());
                    self.read_config(config);
                }
                #[cfg(not(target_arch = "wasm32"))]
                AppImageAction::AddRecent(path) => self.add_recent(path),
                AppImageAction::Replace(new_images) => {
                    rebuild = true;
                    check_names = true;
                    for image in &new_images {
                        ctx.include_bytes(image.thumbnail_uri(), image.thumbnail.clone());
                    }
                    self.data.image_data = new_images;
                }
                AppImageAction::UpdateSpriteSheet(generation, _)
                    if generation != BUILD_GENERATION.load(std::sync::atomic::Ordering::SeqCst) => {
                }
                AppImageAction::UpdateSpriteSheet(_, outcome) => {
                    self.building = false;
                    self.min_size = outcome.min_size;
                    self.json_preview.invalidate();
                    match outcome.result {
                        Ok(built) => {
                            self.last_error = None;
                            ctx.forget_image(preview::PREVIEW_URI);
                            ctx.include_bytes(preview::PREVIEW_URI, built.png.clone());
                            self.output = SpriteSheetState::Ok(built);
                        }
                        Err(error) => {
                            self.last_error = Some(error);
                            self.output = SpriteSheetState::Empty;
                        }
                    }
                }
            }
        }
        if duplicates > 0 {
            notify(
                Level::Info,
                format!("Skipped {duplicates} image(s) that were already added"),
            );
        }
        if rebuild {
            let colliding = self.rebuild_image_data();
            if check_names && colliding > 0 {
                notify(
                    Level::Warning,
                    format!("{colliding} image(s) share a frame name with another image"),
                );
            }
            let paths: HashSet<&str> = self
                .data
                .image_data
                .iter()
                .map(|image| image.path.as_str())
                .collect();
            self.selected.retain(|path| paths.contains(path.as_str()));
            self.selection_anchor = None;
            self.build_atlas();
        }
    }

    fn collect_shortcuts(&self, ctx: &Context) -> Vec<Action> {
        let typing = ctx.egui_wants_keyboard_input();
        let mut actions = Vec::new();
        ctx.input_mut(|i| {
            // Shortcuts with more modifiers go first, otherwise they would match the shorter ones.
            if i.consume_shortcut(&SAVE_JSON) {
                actions.push(Action::SaveJson);
            }
            if i.consume_shortcut(&SAVE_ATLAS) {
                actions.push(Action::SaveAtlas);
            }
            if i.consume_shortcut(&OPEN) {
                actions.push(Action::Open);
            }
            if typing {
                return;
            }
            if i.consume_shortcut(&REDO) || i.consume_shortcut(&REDO_ALT) {
                actions.push(Action::Redo);
            }
            if i.consume_shortcut(&UNDO) {
                actions.push(Action::Undo);
            }
            if i.consume_shortcut(&SELECT_ALL) {
                actions.push(Action::SelectAll);
            }
            if i.consume_shortcut(&REMOVE_SELECTED) || i.consume_shortcut(&REMOVE_SELECTED_ALT) {
                actions.push(Action::RemoveSelected);
            }
        });
        actions
    }

    fn run_action(&mut self, action: Action) {
        let can_save = self.output.is_ok() && !self.building;
        match action {
            Action::Open => self.read_files(),
            Action::SaveAtlas if can_save => self.save_atlas(),
            Action::SaveJson if can_save => self.save_json(),
            #[cfg(not(target_arch = "wasm32"))]
            Action::SaveConfig if !self.data.image_data.is_empty() => self.save_config(),
            Action::Undo => {
                if let Some(state) = self.undoer.undo(&self.data) {
                    self.data = state.clone();
                    push_cmd(AppImageAction::RebuildAtlas);
                }
            }
            Action::Redo => {
                if let Some(state) = self.undoer.redo(&self.data) {
                    self.data = state.clone();
                    push_cmd(AppImageAction::RebuildAtlas);
                }
            }
            Action::SelectAll => {
                self.selected = self
                    .data
                    .image_data
                    .iter()
                    .map(|i| i.path.clone())
                    .collect();
            }
            Action::RemoveSelected if !self.selected.is_empty() => {
                push_cmd(AppImageAction::Remove(
                    self.selected.iter().cloned().collect(),
                ));
            }
            Action::Clear if !self.data.image_data.is_empty() => push_cmd(AppImageAction::Clear),
            Action::FitPreview => self.preview.fit(),
            _ => {}
        }
    }

    fn update_window_title(&mut self, ctx: &Context) {
        let title = if self.data.image_data.is_empty() {
            "rPack".to_owned()
        } else {
            format!("rPack - {}", self.data.settings.output_path)
        };
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }

    fn request_rebuild(&mut self, ctx: &Context) {
        self.pending_rebuild = Some(ctx.input(|i| i.time) + REBUILD_DEBOUNCE);
        ctx.request_repaint_after(Duration::from_secs_f64(REBUILD_DEBOUNCE));
    }

    fn top_panel(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        profile_scope!("top_panel");
        let can_save = self.output.is_ok() && !self.building;
        let has_images = !self.data.image_data.is_empty();
        let ctx = ui.ctx().clone();
        egui::Panel::top("topPanel")
            .frame(egui::Frame::canvas(ui.style()))
            .show(ui, |ui| {
                let title_font = FontId::new(26.0, FontFamily::Name("semibold".into()));
                ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    ui.add(
                        Label::new(
                            RichText::new("rPack")
                                .font(title_font)
                                .color(MY_ACCENT_COLOR32)
                                .strong(),
                        )
                        .selectable(false),
                    );
                    ui.add_space(10.0);
                    egui::MenuBar::new().ui(ui, |ui| {
                        ui.menu_button("File", |ui| {
                            if menu_item(
                                ui,
                                ICON_ADD_PHOTO_ALTERNATE,
                                "Add images...",
                                Some(&OPEN),
                                true,
                            ) {
                                actions.push(Action::Open);
                            }
                            if menu_item(
                                ui,
                                ICON_SAVE,
                                "Save atlas image",
                                Some(&SAVE_ATLAS),
                                can_save,
                            ) {
                                actions.push(Action::SaveAtlas);
                            }
                            if menu_item(
                                ui,
                                ICON_SAVE_AS,
                                "Save atlas json",
                                Some(&SAVE_JSON),
                                can_save,
                            ) {
                                actions.push(Action::SaveJson);
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                if menu_item(
                                    ui,
                                    ICON_DESCRIPTION,
                                    "Save config...",
                                    None,
                                    has_images,
                                ) {
                                    actions.push(Action::SaveConfig);
                                }
                                ui.add_enabled_ui(!self.last_editor_paths.is_empty(), |ui| {
                                    ui.menu_button(
                                        icon_label(ICON_FOLDER_OPEN, "Open recent"),
                                        |ui| {
                                            let mut chosen = None;
                                            for path in &self.last_editor_paths {
                                                if ui.button(path).clicked() {
                                                    chosen = Some(path.clone());
                                                    ui.close();
                                                }
                                            }
                                            if let Some(path) = chosen {
                                                self.open_recent(&path);
                                            }
                                        },
                                    );
                                });
                            }
                        });
                        ui.menu_button("Edit", |ui| {
                            let can_undo = self.undoer.has_undo(&self.data);
                            let can_redo = self.undoer.has_redo(&self.data);
                            if menu_item(ui, ICON_UNDO, "Undo", Some(&UNDO), can_undo) {
                                actions.push(Action::Undo);
                            }
                            if menu_item(ui, ICON_REDO, "Redo", Some(&REDO), can_redo) {
                                actions.push(Action::Redo);
                            }
                            ui.separator();
                            if menu_item(
                                ui,
                                ICON_SELECT_ALL,
                                "Select all images",
                                Some(&SELECT_ALL),
                                has_images,
                            ) {
                                actions.push(Action::SelectAll);
                            }
                            if menu_item(
                                ui,
                                ICON_DELETE,
                                "Remove selected",
                                Some(&REMOVE_SELECTED),
                                !self.selected.is_empty(),
                            ) {
                                actions.push(Action::RemoveSelected);
                            }
                            if menu_item(
                                ui,
                                ICON_DELETE_SWEEP,
                                "Remove all images",
                                None,
                                has_images,
                            ) {
                                actions.push(Action::Clear);
                            }
                        });
                        ui.menu_button("View", |ui| {
                            ui.checkbox(&mut self.view_settings.display_json, "Show atlas JSON");
                            ui.checkbox(
                                &mut self.view_settings.show_outlines,
                                "Show sprite outlines",
                            );
                            if menu_item(
                                ui,
                                ICON_FIT_SCREEN,
                                "Fit preview",
                                None,
                                self.output.is_ok(),
                            ) {
                                actions.push(Action::FitPreview);
                            }
                            ui.separator();
                            ui.label("Interface scale");
                            ui.add(Slider::new(&mut self.view_settings.ui_scale, 1.0..=2.0));
                        });
                    });
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(10.0);
                        if ui
                            .button(icon_label(ICON_ADD_PHOTO_ALTERNATE, "Add images"))
                            .on_hover_text(hint(&ctx, "Add images or open a config", &OPEN))
                            .clicked()
                        {
                            actions.push(Action::Open);
                        }
                        if ui
                            .add_enabled(can_save, Button::new(icon_label(ICON_SAVE, "Save image")))
                            .on_hover_text(hint(&ctx, "Save the atlas image", &SAVE_ATLAS))
                            .clicked()
                        {
                            actions.push(Action::SaveAtlas);
                        }
                        if ui
                            .add_enabled(
                                can_save,
                                Button::new(icon_label(ICON_DATA_OBJECT, "Save JSON")),
                            )
                            .on_hover_text(hint(&ctx, "Save the atlas json", &SAVE_JSON))
                            .clicked()
                        {
                            actions.push(Action::SaveJson);
                        }
                    });
                });
            });
    }

    fn bottom_panel(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        profile_scope!("bottom_panel");
        let ctx = ui.ctx().clone();
        egui::Panel::bottom("bottom_panel")
            .frame(egui::Frame::canvas(ui.style()))
            .show(ui, |ui| {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.add_space(5.0);
                    let can_undo = self.undoer.has_undo(&self.data);
                    let can_redo = self.undoer.has_redo(&self.data);
                    if ui
                        .add_enabled(can_undo, Button::new(icon_label(ICON_UNDO, "Undo")))
                        .on_hover_text(hint(&ctx, "Undo the last change", &UNDO))
                        .clicked()
                    {
                        actions.push(Action::Undo);
                    }
                    if ui
                        .add_enabled(can_redo, Button::new(icon_label(ICON_REDO, "Redo")))
                        .on_hover_text(hint(&ctx, "Redo", &REDO))
                        .clicked()
                    {
                        actions.push(Action::Redo);
                    }
                    ui.add_space(5.0);
                    if let SpriteSheetState::Ok(built) = &self.output {
                        let [width, height] = built.atlas_asset.size;
                        let used: u64 = built
                            .atlas_asset
                            .frames
                            .iter()
                            .map(|f| u64::from(f.frame.w) * u64::from(f.frame.h))
                            .sum();
                        let total = (u64::from(width) * u64::from(height)).max(1);
                        ui.label(format!(
                            "{} sprites  |  {}x{}  |  {:.0}% used",
                            built.atlas_asset.frames.len(),
                            width,
                            height,
                            used as f64 * 100.0 / total as f64
                        ));
                    }
                    powered_by_egui_and_eframe(ui);
                });
                ui.add_space(5.0);
            });
    }

    fn json_panel(&mut self, ui: &mut Ui) {
        let show = self.view_settings.display_json && self.output.is_ok();
        if !show {
            return;
        }
        let SpriteSheetState::Ok(built) = &self.output else {
            return;
        };
        egui::Panel::bottom("json_panel")
            .resizable(true)
            .default_size(200.0)
            .frame(egui::Frame::canvas(ui.style()).inner_margin(10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Atlas JSON").strong());
                    if ui
                        .button(icon_label(ICON_CONTENT_COPY, "Copy"))
                        .on_hover_text("Copy the JSON to the clipboard")
                        .clicked()
                    {
                        ui.ctx().copy_text(built.atlas_asset_json.to_string());
                        notify(Level::Info, "JSON copied to the clipboard");
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("json_vertical_scroll")
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        self.json_preview.show(ui, &built.atlas_asset_json);
                    });
            });
    }

    fn right_panel(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        profile_scope!("right_panel");
        let ctx = ui.ctx().clone();
        let mut show_right_panel = !self.data.image_data.is_empty();
        egui::Panel::right("right")
            .min_size(220.0)
            .max_size(420.0)
            .default_size(280.0)
            .frame(egui::Frame::canvas(ui.style()).inner_margin(10))
            .show_collapsible(ui, &mut show_right_panel, |ui| {
                ui.with_layout(
                    Layout::top_down(egui::Align::Min).with_cross_justify(true),
                    |ui| {
                        ui.style_mut().interaction.selectable_labels = false;
                        self.settings_grid(ui, &ctx);
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.heading(format!("Images ({})", self.data.image_data.len()));
                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .button(icon_label(ICON_DELETE_SWEEP, "Clear"))
                                    .on_hover_text("Remove all images")
                                    .clicked()
                                {
                                    actions.push(Action::Clear);
                                }
                                if ui
                                    .button(icon_label(ICON_ADD, "Add"))
                                    .on_hover_text(hint(&ctx, "Add more images", &OPEN))
                                    .clicked()
                                {
                                    actions.push(Action::Open);
                                }
                            });
                        });
                        ui.separator();
                        self.image_list(ui, &ctx, actions);
                    },
                );
            });
    }

    fn settings_grid(&mut self, ui: &mut Ui, ctx: &Context) {
        profile_scope!("settings_grid");
        let mut changed = false;
        Grid::new("settings_grid")
            .num_columns(2)
            .spacing((8.0, 10.0))
            .striped(true)
            .show(ui, |ui| {
                let id = ui.label("File name").id;
                if ui
                    .text_edit_singleline(&mut self.data.settings.output_path)
                    .labelled_by(id)
                    .on_hover_text("Name of the saved files, without the extension")
                    .changed()
                    && let SpriteSheetState::Ok(data) = &mut self.output
                {
                    data.atlas_asset.filename = format!("{}.png", self.data.settings.output_path);
                    data.rebuild_json();
                    self.json_preview.invalidate();
                }
                ui.end_row();

                ui.label("Output size")
                    .on_hover_text("Maximum side of the square atlas texture");
                let selected_size = match self.data.settings.size {
                    Some(size) => format!("{size}x{size}"),
                    None => format!("Auto (up to {DEFAULT_ATLAS_SIZE})"),
                };
                let min_size = self.min_size;
                let size_changed =
                    MenuButton::from_button(Button::new(selected_size).frame_when_inactive(true))
                        .ui(ui, |ui| {
                            let mut chosen = false;
                            if ui
                                .add(
                                    Button::new(format!("Auto (up to {DEFAULT_ATLAS_SIZE})"))
                                        .selected(self.data.settings.size.is_none()),
                                )
                                .clicked()
                            {
                                self.data.settings.size = None;
                                chosen = true;
                            }
                            for size in ATLAS_SIZES {
                                let same_size = self.data.settings.size == Some(size);
                                if ui
                                    .add_enabled(
                                        size >= min_size,
                                        Button::new(format!("{size}x{size}")).selected(same_size),
                                    )
                                    .on_disabled_hover_text("Too small for the current images")
                                    .clicked()
                                {
                                    self.data.settings.size = Some(size);
                                    chosen = true;
                                }
                            }
                            if chosen {
                                ui.close();
                            }
                            chosen
                        })
                        .1
                        .is_some_and(|response| response.inner);
                changed |= size_changed;
                ui.end_row();

                changed |= slider_field(
                    ui,
                    "Border padding",
                    "Space in pixels around the whole atlas",
                    &mut self.data.settings.border_padding,
                    0,
                    0..=10,
                );
                ui.end_row();
                changed |= slider_field(
                    ui,
                    "Texture padding",
                    "Space in pixels between sprites",
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
                ui.label("Skip metadata")
                    .on_hover_text("Leave the metadata section out of the saved json");
                if ui.checkbox(&mut skip_metadata, "").changed() {
                    self.data.settings.skip_serializing_metadata = Some(skip_metadata);
                    if let SpriteSheetState::Ok(data) = &mut self.output {
                        data.atlas_asset.metadata.skip_serialization = skip_metadata;
                        data.rebuild_json();
                        self.json_preview.invalidate();
                    }
                }
            });
        if changed {
            self.request_rebuild(ctx);
        }
    }

    fn image_list(&mut self, ui: &mut Ui, ctx: &Context, actions: &mut Vec<Action>) {
        profile_scope!("image_list");
        let mut row_click: Option<(usize, Modifiers)> = None;
        let mut remove: Vec<String> = Vec::new();
        let length = self.data.image_data.len();
        let selected = &self.selected;
        let images = &self.data.image_data;
        let hovered_frame = self.hovered_frame.as_deref();
        // Bring the sprite hovered in the preview into view, once per hover.
        let scroll_to = hovered_frame
            .filter(|id| self.scrolled_to.as_deref() != Some(*id))
            .and_then(|id| images.iter().position(|image| image.id() == id));
        self.scrolled_to = self.hovered_frame.clone();
        let remove_ref = &mut remove;
        let click_ref = &mut row_click;
        ui.add_enabled_ui(!self.building, |ui| {
            let mut table = TableBuilder::new(ui)
                .striped(true)
                .vscroll(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(70.0))
                .column(Column::remainder().at_least(50.0))
                .column(Column::exact(32.0))
                .sense(egui::Sense::click());
            if let Some(index) = scroll_to {
                table = table.scroll_to_row(index, None);
            }
            table.body(|body| {
                body.rows(64.0, length, |mut row| {
                    let index = row.index();
                    let file = &images[index];
                    row.set_selected(selected.contains(&file.path));
                    row.set_hovered(hovered_frame == Some(file.id()));
                    row.col(|ui| {
                        ui.vertical_centered_justified(|ui| {
                            ui.add_sized(
                                (64.0, 64.0),
                                Image::from_uri(file.thumbnail_uri()).corner_radius(5u8),
                            )
                            .on_hover_text(format!(
                                "{}x{}\n{}",
                                file.width, file.height, file.path
                            ));
                        });
                    });
                    row.col(|ui| {
                        ui.add(
                            Label::new(file.id())
                                .selectable(false)
                                .wrap_mode(egui::TextWrapMode::Truncate),
                        );
                    });
                    row.col(|ui| {
                        if ui
                            .add(Button::new(ICON_CLOSE).frame(false))
                            .on_hover_text("Remove")
                            .clicked()
                        {
                            remove_ref.push(file.path.clone());
                        }
                    });
                    let response = row.response();
                    if response.clicked() {
                        *click_ref = Some((index, ctx.input(|i| i.modifiers)));
                    }
                    response.context_menu(|ui| {
                        if ui.button(icon_label(ICON_DELETE, "Remove")).clicked() {
                            if selected.contains(&file.path) {
                                remove_ref.extend(selected.iter().cloned());
                            } else {
                                remove_ref.push(file.path.clone());
                            }
                            ui.close();
                        }
                        if ui
                            .button(icon_label(ICON_SELECT_ALL, "Select all"))
                            .clicked()
                        {
                            actions.push(Action::SelectAll);
                            ui.close();
                        }
                    });
                });
            });
        });
        if !remove.is_empty() {
            push_cmd(AppImageAction::Remove(remove));
        }
        if let Some((index, modifiers)) = row_click {
            let path = self.data.image_data[index].path.clone();
            if modifiers.shift
                && let Some(anchor) = self.selection_anchor
            {
                let (from, to) = (anchor.min(index), anchor.max(index));
                if !modifiers.command {
                    self.selected.clear();
                }
                self.selected.extend(
                    self.data.image_data[from..=to.min(length - 1)]
                        .iter()
                        .map(|i| i.path.clone()),
                );
            } else if modifiers.command {
                if !self.selected.remove(&path) {
                    self.selected.insert(path);
                }
                self.selection_anchor = Some(index);
            } else {
                self.selected.clear();
                self.selected.insert(path);
                self.selection_anchor = Some(index);
            }
        }
    }

    fn central_panel(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        profile_scope!("central_panel");
        egui::CentralPanel::default()
            .frame(Frame::central_panel(ui.style()).inner_margin(16i8))
            .show(ui, |ui| {
                if let Some(error) = &self.last_error {
                    let text = RichText::new(format!("Error: {error}"))
                        .font(FontId::new(20.0, FontFamily::Name("semibold".into())))
                        .color(ui.visuals().error_fg_color)
                        .strong();
                    ui.add(Label::new(text));
                    ui.add_space(6.0);
                }
                if self.data.image_data.is_empty() {
                    self.empty_state(ui, actions);
                    return;
                }
                if let SpriteSheetState::Ok(built) = &self.output {
                    ui.horizontal(|ui| {
                        if ui.button(icon_label(ICON_FIT_SCREEN, "Fit")).clicked() {
                            self.preview.fit();
                        }
                        if ui.button("100%").clicked() {
                            self.preview.actual_size();
                        }
                        ui.label(format!("{}%", self.preview.zoom_percent()));
                        ui.separator();
                        ui.checkbox(&mut self.view_settings.show_outlines, "Outlines");
                        ui.checkbox(&mut self.view_settings.display_json, "JSON");
                        ui.weak("Scroll to zoom, drag to pan, double click to fit");
                    });
                    ui.add_space(6.0);
                    let selected_keys: HashSet<&str> = self
                        .data
                        .image_data
                        .iter()
                        .filter(|image| self.selected.contains(&image.path))
                        .map(|image| image.id())
                        .collect();
                    let hovered = preview::show(
                        ui,
                        &mut self.preview,
                        built,
                        self.view_settings.show_outlines,
                        self.building,
                        MY_ACCENT_COLOR32,
                        &selected_keys,
                    );
                    if hovered != self.hovered_frame {
                        self.hovered_frame = hovered;
                        ui.ctx().request_repaint();
                    }
                } else if self.building {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.heading(RichText::new("Building atlas...").color(MY_ACCENT_COLOR32));
                        ui.add_space(10.0);
                        ui.spinner();
                    });
                }
            });
    }

    fn empty_state(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        egui::ScrollArea::vertical()
            .id_salt("vertical_scroll")
            .show(ui, |ui| {
                ui.vertical_centered_justified(|ui| {
                    ui.add_space(25.0);
                    Grid::new("Header").num_columns(2).show(ui, |ui| {
                        ui.add_sized((256.0, 256.0), Image::new("bytes://image.png"));
                        ui.vertical_centered_justified(|ui| {
                            ui.add_space(15.0);
                            ui.add(
                                Label::new(RichText::new("Create spritesheets in seconds!").heading())
                                    .selectable(false),
                            );
                            ui.separator();
                            ui.add_space(10.0);
                            ui.add(
                                Label::new("Drag and drop images or a folder here to start creating new spritesheets.")
                                    .selectable(false),
                            );
                            ui.add_space(10.0);
                            if ui
                                .button(icon_label(ICON_ADD_PHOTO_ALTERNATE, "Add images..."))
                                .clicked()
                            {
                                actions.push(Action::Open);
                            }
                        });
                    });
                    ui.add_space(10.0);
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if !self.last_editor_paths.is_empty() {
                            ui.separator();
                            ui.add(Label::new(RichText::new("Recent projects").heading()).selectable(false));
                            ui.add_space(10.0);
                        }
                        let mut chosen = None;
                        for path in &self.last_editor_paths {
                            if ui.add(Button::new(path).frame(false)).clicked() {
                                chosen = Some(path.clone());
                            }
                            ui.add_space(10.0);
                        }
                        if let Some(path) = chosen {
                            self.open_recent(&path);
                        }
                    }
                });
            });
    }

    fn drop_overlay(&self, ctx: &Context) {
        if ctx.input(|i| i.raw.hovered_files.is_empty()) {
            return;
        }
        let rect = ctx.content_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::unique("drop_overlay"),
        ));
        painter.rect_filled(rect, 0.0, Color32::from_black_alpha(160));
        painter.rect_stroke(
            rect.shrink(12.0),
            12.0,
            egui::Stroke::new(3.0, MY_ACCENT_COLOR32),
            egui::StrokeKind::Inside,
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Drop images, a folder or a config here",
            FontId::new(28.0, FontFamily::Proportional),
            Color32::WHITE,
        );
    }
}

impl eframe::App for Application {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.last_editor_paths);
        eframe::set_value(storage, "view_settings", &self.view_settings);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        {
            profile_scope!("handle_undo");
            self.undoer
                .feed_state(ctx.input(|input| input.time), &self.data);
        }
        if (self.view_settings.ui_scale - self.applied_scale).abs() > f32::EPSILON {
            apply_style(&ctx, self.view_settings.ui_scale);
            self.applied_scale = self.view_settings.ui_scale;
        }
        if let Some(at) = self.pending_rebuild {
            let now = ctx.input(|i| i.time);
            if now >= at {
                self.pending_rebuild = None;
                push_cmd(AppImageAction::RebuildAtlas);
            } else {
                ctx.request_repaint_after(Duration::from_secs_f64(at - now));
            }
        }
        self.handle_dropped_files(&ctx);
        self.drain_commands(&ctx);
        self.update_window_title(&ctx);

        let mut actions = self.collect_shortcuts(&ctx);
        self.top_panel(ui, &mut actions);
        self.bottom_panel(ui, &mut actions);
        self.json_panel(ui);
        self.right_panel(ui, &mut actions);
        self.central_panel(ui, &mut actions);
        self.drop_overlay(&ctx);
        self.notifications.show(&ctx);
        for action in actions {
            self.run_action(action);
        }
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.with_layout(Layout::right_to_left(egui::Align::Min), |ui| {
        ui.add_space(10.0);
        if ui
            .add(Button::new(VERSION).frame(false))
            .on_hover_text(GIT_HASH)
            .clicked()
        {
            ui.ctx()
                .open_url(egui::OpenUrl::new_tab(env!("CARGO_PKG_HOMEPAGE")));
        }
        ui.separator();
        theme_buttons(ui);
        ui.separator();
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.hyperlink_to("Mev Lyshkin", "https://www.mevlyshkin.com/");
        ui.add_space(10.0);
        ui.label("Made by");
        ui.add_space((ui.available_width() - 10.0).max(15.0));
    });
}

/// Theme switcher using Material icons, since the default one relies on glyphs that may be missing.
fn theme_buttons(ui: &mut Ui) {
    let current = ui.options(|opt| opt.theme_preference);
    let mut chosen = current;
    for (preference, icon, tooltip) in [
        (egui::ThemePreference::Dark, ICON_DARK_MODE, "Dark theme"),
        (egui::ThemePreference::Light, ICON_LIGHT_MODE, "Light theme"),
        (
            egui::ThemePreference::System,
            ICON_SETTINGS_BRIGHTNESS,
            "Follow the system theme",
        ),
    ] {
        if ui
            .add(Button::new(icon).selected(current == preference))
            .on_hover_text(tooltip)
            .clicked()
        {
            chosen = preference;
        }
    }
    if chosen != current {
        ui.ctx().set_theme(chosen);
    }
}

fn slider_field(
    ui: &mut Ui,
    label: &str,
    tooltip: &str,
    field: &mut Option<u32>,
    default_value: u32,
    range: RangeInclusive<u32>,
) -> bool {
    ui.label(label).on_hover_text(tooltip);
    let mut value = field.unwrap_or(default_value);
    if ui.add(Slider::new(&mut value, range)).changed() {
        *field = Some(value);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod packing_tests {
    use super::*;

    fn image(width: u32, height: u32) -> AppImageData {
        AppImageData::new(
            "tiles/tile.png".into(),
            "tile".into(),
            image::DynamicImage::new_rgba8(width, height),
        )
    }

    fn min_size(settings: &TilemapGenerationConfig, images: &[AppImageData]) -> (u32, bool) {
        let dimensions: Vec<_> = images.iter().map(|i| (i.id(), i.width, i.height)).collect();
        let (size, error) = find_min_size(settings.into(), &dimensions);
        (size, error.is_some())
    }

    #[test]
    fn minimum_size_matches_builder_and_reports_exhaustion() {
        let settings = TilemapGenerationConfig {
            border_padding: Some(1),
            ..Default::default()
        };
        let small = vec![image(30, 30)];
        assert_eq!(min_size(&settings, &small), (32, false));
        let config = rpack_cli::packing::generation_config(Some(32), None, Some(1)).unwrap();
        let file = ImageFile {
            id: small[0].id.clone(),
            image: (*small[0].image).clone(),
        };
        assert!(Spritesheet::build(config, &[file], "atlas.png").is_ok());
        let huge = vec![image(4096, 1)];
        assert_eq!(min_size(&settings, &huge), (0, true));
    }

    #[test]
    fn build_produces_png_preview() {
        let inputs = vec![BuildInput {
            id: "tile".into(),
            image: Arc::new(image::DynamicImage::new_rgba8(8, 8)),
            width: 8,
            height: 8,
        }];
        let outcome = run_build(TilemapGenerationConfig::default(), inputs);
        let built = outcome.result.unwrap_or_else(|_| panic!("build failed"));
        assert_eq!(outcome.min_size, 32);
        assert!(built.png.starts_with(&[0x89, b'P', b'N', b'G']));
    }

    #[test]
    fn remove_and_duplicate_paths_are_handled_by_path() {
        let mut a = image(4, 4);
        a.path = "dir/a.png".into();
        let mut b = image(4, 4);
        b.path = "dir/b.png".into();
        let mut app = Application::default();
        app.data.image_data = vec![a, b];
        assert_eq!(app.rebuild_image_data(), 0);
        app.data.image_data[1].id = app.data.image_data[0].id.clone();
        assert_eq!(
            app.data
                .image_data
                .iter()
                .map(|i| i.id())
                .collect::<HashSet<_>>()
                .len(),
            1
        );
    }

    #[test]
    fn short_name_handles_both_separators() {
        assert_eq!(short_name("a/b/c.png"), "c.png");
        assert_eq!(short_name("a\\b\\c.png"), "c.png");
        assert_eq!(short_name("c.png"), "c.png");
    }
}
