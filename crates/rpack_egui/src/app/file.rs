//! Adding images and saving results.
use super::Application;
use crate::platform::{self, SaveRequest};
use crate::state::AppImageData;
use crate::ui::toasts::Level;

#[cfg(not(target_arch = "wasm32"))]
const MAX_RECENT_PROJECTS: usize = 8;

impl Application {
    pub(super) fn add_images(&self) {
        let working_dir = std::path::absolute(self.state.project.common_prefix())
            .map_or(String::from("."), |p| p.to_string_lossy().to_string());
        platform::pick_images(self.sender.clone(), working_dir);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn pick_config(&self) {
        platform::pick_config(self.sender.clone());
    }

    pub(super) fn save_atlas(&self) {
        let Some(built) = &self.state.build.output else {
            return;
        };
        platform::save_file(
            self.sender.clone(),
            SaveRequest {
                file_name: format!("{}.png", self.state.project.settings.output_path),
                filter: None,
                bytes: built.png.clone(),
                what: "atlas image",
            },
        );
    }

    pub(super) fn save_json(&self) {
        let Some(built) = &self.state.build.output else {
            return;
        };
        let data = serde_json::to_string_pretty(&built.atlas_asset_json)
            .unwrap_or_else(|_| built.atlas_asset_json.to_string());
        platform::save_file(
            self.sender.clone(),
            SaveRequest {
                file_name: format!("{}.rpack.json", self.state.project.settings.output_path),
                filter: Some((".rpack.json", &["rpack.json"])),
                bytes: data.into_bytes().into(),
                what: "atlas json",
            },
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn save_config(&self) {
        let paths = self
            .state
            .project
            .image_data
            .iter()
            .map(|image| image.path.clone())
            .collect();
        platform::save_config(
            self.sender.clone(),
            self.state.project.settings.clone(),
            paths,
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn add_recent(&mut self, path: String) {
        let recents = &mut self.state.ui.settings.recents;
        recents.retain(|p| *p != path);
        recents.insert(0, path);
        recents.truncate(MAX_RECENT_PROJECTS);
    }

    /// Adds images to the project, skipping paths that are already in it.
    pub(super) fn merge_images(&mut self, ctx: &egui::Context, images: Vec<AppImageData>) {
        let mut duplicates = 0;
        for image in images {
            let project = &mut self.state.project;
            if project.image_data.iter().any(|i| i.path == image.path) {
                duplicates += 1;
                continue;
            }
            ctx.include_bytes(image.thumbnail_uri(), image.thumbnail.clone());
            project.image_data.push(image);
        }
        if duplicates > 0 {
            self.state.ui.toasts.push(
                Level::Info,
                format!("Skipped {duplicates} image(s) that were already added"),
            );
        }
        self.update_ids_and_warn();
    }

    /// Replaces the project with a config and the images it refers to.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn replace_project(
        &mut self,
        ctx: &egui::Context,
        settings: rpack_core::TilemapGenerationConfig,
        images: Vec<AppImageData>,
    ) {
        for image in &images {
            ctx.include_bytes(image.thumbnail_uri(), image.thumbnail.clone());
        }
        self.state.project.settings = settings;
        self.state.project.image_data = images;
        self.update_ids_and_warn();
    }

    fn update_ids_and_warn(&mut self) {
        let colliding = self.state.project.update_ids();
        if colliding > 0 {
            self.state.ui.toasts.push(
                Level::Warning,
                format!("{colliding} image(s) share a frame name with another image"),
            );
        }
    }
}
