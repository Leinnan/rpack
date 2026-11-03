//! Keeps the atlas up to date with the project.
use super::Application;
use crate::events::AppEvent;
use crate::platform::jobs;
use crate::state::{BuildOutcome, Built};
use crate::ui::preview;
use rpack_core::pipeline::{self, ATLAS_SIZES};
use rpack_core::{SpritesheetError, TilemapGenerationConfig};
use std::sync::Arc;
use std::time::Duration;

const REBUILD_DEBOUNCE: f64 = 0.15;

struct BuildInput {
    id: String,
    image: Arc<image::DynamicImage>,
}

/// Runs on a worker thread: finds the minimum size, packs and encodes the preview.
fn run_build(settings: &TilemapGenerationConfig, inputs: &[BuildInput]) -> BuildOutcome {
    let images: Vec<_> = inputs
        .iter()
        .map(|input| (input.id.as_str(), &*input.image))
        .collect();
    let filename = format!("{}.png", settings.output_path);
    let build = pipeline::build_atlas(settings, &images, &filename);
    let result = build.result.and_then(|sheet| {
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
    BuildOutcome {
        min_size: build.min_size,
        result,
    }
}

impl Application {
    /// Notices project changes each frame and starts debounced rebuilds.
    pub(super) fn sync_build(&mut self, ctx: &egui::Context) {
        let key = self.state.project.pack_key();
        let now = ctx.input(|i| i.time);
        if self.state.build.last_key != Some(key) {
            self.state.build.last_key = Some(key);
            self.state.build.pending_at = Some(now + REBUILD_DEBOUNCE);
            let images = &self.state.project.image_data;
            self.state.ui.prune_selection(images);
        }
        if let Some(at) = self.state.build.pending_at {
            if now >= at {
                self.state.build.pending_at = None;
                self.start_build();
            } else {
                ctx.request_repaint_after(Duration::from_secs_f64(at - now));
            }
        }
        self.sync_output_names();
    }

    /// Applies what does not change the layout to the output, so it needs no rebuild.
    /// Also covers a build that started before the change and has just arrived.
    fn sync_output_names(&mut self) {
        let settings = &self.state.project.settings;
        let Some(output) = &mut self.state.build.output else {
            return;
        };
        let filename = format!("{}.png", settings.output_path);
        let skip_metadata = settings.skip_serializing_metadata.unwrap_or_default();
        if output.atlas_asset.filename != filename
            || output.atlas_asset.metadata.skip_serialization != skip_metadata
        {
            output.atlas_asset.filename = filename;
            output.atlas_asset.metadata.skip_serialization = skip_metadata;
            output.rebuild_json();
            self.state.ui.json_preview.invalidate();
        }
    }

    fn start_build(&mut self) {
        let build = &mut self.state.build;
        build.generation += 1;
        if self.state.project.image_data.is_empty() {
            build.output = None;
            build.building = false;
            build.rerun = false;
            build.last_error = None;
            build.min_size = ATLAS_SIZES[0];
            self.state.ui.json_preview.invalidate();
            return;
        }
        build.building = true;
        if build.in_flight {
            build.rerun = true;
            return;
        }
        build.in_flight = true;
        let generation = build.generation;
        let settings = self.state.project.settings.clone();
        let inputs: Vec<BuildInput> = self
            .state
            .project
            .image_data
            .iter()
            .map(|image| BuildInput {
                id: image.id.clone(),
                image: image.image.clone(),
            })
            .collect();
        let tx = self.sender.clone();
        jobs::spawn(move || {
            tx.send(AppEvent::Build(generation, run_build(&settings, &inputs)));
        });
    }

    pub(super) fn finish_build(
        &mut self,
        ctx: &egui::Context,
        generation: u64,
        outcome: BuildOutcome,
    ) {
        let build = &mut self.state.build;
        build.in_flight = false;
        if build.rerun {
            build.rerun = false;
            self.start_build();
            return;
        }
        if generation != build.generation {
            return;
        }
        build.building = false;
        build.min_size = outcome.min_size;
        match outcome.result {
            Ok(built) => {
                build.last_error = None;
                ctx.forget_image(preview::PREVIEW_URI);
                ctx.include_bytes(preview::PREVIEW_URI, built.png.clone());
                build.output = Some(built);
            }
            Err(error) => {
                build.last_error = Some(error);
                build.output = None;
            }
        }
        self.state.ui.json_preview.invalidate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_produces_png_preview() {
        let inputs = [BuildInput {
            id: "tile".into(),
            image: Arc::new(image::DynamicImage::new_rgba8(8, 8)),
        }];
        let outcome = run_build(&TilemapGenerationConfig::default(), &inputs);
        let built = outcome.result.unwrap_or_else(|_| panic!("build failed"));
        assert_eq!(outcome.min_size, 32);
        assert!(built.png.starts_with(&[0x89, b'P', b'N', b'G']));
    }
}
