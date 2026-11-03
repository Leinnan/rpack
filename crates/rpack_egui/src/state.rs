//! Everything the panels render from.
use crate::settings::Settings;
use crate::ui::json_preview::JsonPreview;
use crate::ui::preview::PreviewState;
use crate::ui::toasts::Notifications;
use egui::util::undoer::Undoer;
use rpack_core::{Spritesheet, SpritesheetError, TilemapGenerationConfig, packing};
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use texture_packer::importer::ImageImporter;

const DEFAULT_OUTPUT_NAME: &str = "atlas";

#[derive(Clone)]
pub struct AppImageData {
    pub width: u32,
    pub height: u32,
    pub id: String,
    pub image: Arc<image::DynamicImage>,
    pub path: String,
    pub thumbnail: Arc<[u8]>,
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
    pub fn new(path: String, image: image::DynamicImage) -> Self {
        let mut thumbnail = vec![];
        let _ = image.thumbnail(64, 64).write_to(
            &mut std::io::Cursor::new(&mut thumbnail),
            image::ImageFormat::Png,
        );
        Self {
            width: image.width(),
            height: image.height(),
            id: packing::frame_key(&path),
            image: Arc::new(image),
            path,
            thumbnail: thumbnail.into(),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn from_bytes(bytes: &[u8], path: String) -> Option<Self> {
        let image = ImageImporter::import_from_memory(bytes).ok()?;
        Some(Self::new(path, image))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_file(path: &std::path::Path) -> Option<Self> {
        let image = ImageImporter::import_from_file(path).ok()?;
        Some(Self::new(path.display().to_string(), image))
    }

    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    fn update_id(&mut self, prefix: &str) {
        let path = self.path.replace('\\', "/");
        self.id = packing::frame_key(path.strip_prefix(prefix).unwrap_or(&path));
    }

    pub fn thumbnail_uri(&self) -> String {
        format!("bytes://{}", self.path)
    }
}

/// State covered by undo/redo.
#[derive(Clone, PartialEq)]
pub struct Project {
    pub image_data: Vec<AppImageData>,
    pub settings: TilemapGenerationConfig,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            image_data: Vec::new(),
            settings: TilemapGenerationConfig {
                output_path: DEFAULT_OUTPUT_NAME.to_owned(),
                ..Default::default()
            },
        }
    }
}

impl Project {
    pub fn common_prefix(&self) -> String {
        let file_paths: Vec<&str> = self
            .image_data
            .iter()
            .map(|image| image.path.as_str())
            .collect();
        rpack_core::get_common_prefix(&file_paths)
    }

    /// Recomputes frame ids and returns how many of them are not unique.
    pub fn update_ids(&mut self) -> usize {
        let prefix = self.common_prefix();
        self.image_data
            .iter_mut()
            .for_each(|image| image.update_id(prefix.as_str()));
        let mut seen = HashSet::new();
        self.image_data
            .iter()
            .filter(|image| !seen.insert(image.id()))
            .count()
    }

    /// Hash of everything that changes the packed layout. A different key means the atlas
    /// has to be rebuilt; the file name and metadata flag can be patched in instead.
    pub fn pack_key(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.settings.size.hash(&mut hasher);
        self.settings.border_padding.hash(&mut hasher);
        self.settings.texture_padding.hash(&mut hasher);
        for image in &self.image_data {
            image.path.hash(&mut hasher);
            image.id.hash(&mut hasher);
            image.width.hash(&mut hasher);
            image.height.hash(&mut hasher);
        }
        hasher.finish()
    }
}

/// A finished atlas together with its PNG encoding, which is used for the preview and saving.
#[derive(Clone)]
pub struct Built {
    pub sheet: Spritesheet,
    pub png: Arc<[u8]>,
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

pub struct BuildOutcome {
    /// Smallest square atlas side that fits all images, `0` if even the largest does not.
    pub min_size: u32,
    pub result: Result<Built, SpritesheetError>,
}

#[derive(Default)]
pub struct BuildState {
    /// Last successfully built atlas. It stays visible (dimmed) while a new one is building.
    pub output: Option<Built>,
    pub building: bool,
    pub min_size: u32,
    pub last_error: Option<SpritesheetError>,
    /// Id of the most recently started build; results of older builds are discarded.
    pub generation: u64,
    /// A worker is running. Only one runs at a time.
    pub in_flight: bool,
    /// The project changed while a worker was running, so another build has to follow it.
    pub rerun: bool,
    /// Time after which a debounced rebuild is started.
    pub pending_at: Option<f64>,
    /// `Project::pack_key` of the last change that was noticed.
    pub last_key: Option<u64>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    About,
}

#[derive(Default)]
pub struct UiState {
    /// Paths of the selected images.
    pub selected: HashSet<String>,
    pub selection_anchor: Option<usize>,
    /// Id of the sprite under the cursor in the atlas preview, highlighted in the image list.
    pub hovered_frame: Option<String>,
    /// Last hovered sprite the image list was scrolled to.
    pub scrolled_to: Option<String>,
    pub preview: PreviewState,
    pub json_preview: JsonPreview,
    pub toasts: Notifications,
    pub settings: Settings,
    pub dialog: Option<Dialog>,
    /// The project name in the toolbar is being edited.
    pub renaming: bool,
    /// The window is too narrow for the inspector, so it is hidden unless forced.
    pub narrow: bool,
    /// The inspector was opened by hand while the window is narrow.
    pub inspector_forced: bool,
    /// Scroll position and height of the sprite list in the last frame.
    pub sprites_offset: f32,
    pub sprites_view: f32,
    /// The toolbar shares its row with the macOS traffic lights and moves the window.
    pub native_chrome: bool,
    /// The system menu bar replaces the egui one.
    pub native_menu: bool,
}

impl UiState {
    pub fn inspector_visible(&self) -> bool {
        self.settings.show_inspector && (!self.narrow || self.inspector_forced)
    }

    pub fn toggle_inspector(&mut self) {
        if !self.narrow {
            self.settings.show_inspector = !self.settings.show_inspector;
        } else if self.inspector_visible() {
            self.inspector_forced = false;
        } else {
            self.settings.show_inspector = true;
            self.inspector_forced = true;
        }
    }

    /// Applies a click on a row of the image list: plain, toggling (Cmd), range (Shift)
    /// or additive range (Cmd+Shift).
    pub fn click_sprite(
        &mut self,
        index: usize,
        modifiers: egui::Modifiers,
        images: &[AppImageData],
    ) {
        let path = images[index].path.clone();
        if modifiers.shift
            && let Some(anchor) = self.selection_anchor
        {
            let (from, to) = (anchor.min(index), anchor.max(index));
            if !modifiers.command {
                self.selected.clear();
            }
            self.selected.extend(
                images[from..=to.min(images.len() - 1)]
                    .iter()
                    .map(|image| image.path.clone()),
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

    /// Forgets selected paths that are no longer in the project.
    pub fn prune_selection(&mut self, images: &[AppImageData]) {
        let paths: HashSet<&str> = images.iter().map(|image| image.path.as_str()).collect();
        let before = self.selected.len();
        self.selected.retain(|path| paths.contains(path.as_str()));
        if self.selected.len() != before || self.selection_anchor >= Some(images.len()) {
            self.selection_anchor = None;
        }
    }
}

#[derive(Default)]
pub struct AppState {
    pub project: Project,
    pub build: BuildState,
    pub ui: UiState,
    pub undoer: Undoer<Project>,
}

impl AppState {
    /// The output is up to date with the project, so saving it gives what the user expects.
    pub fn can_save(&self) -> bool {
        self.build.output.is_some() && !self.build.building && self.build.pending_at.is_none()
    }
}
