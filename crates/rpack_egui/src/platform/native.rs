use super::{jobs, report_failed};
use crate::events::{AppEvent, EventSender};
use crate::state::AppImageData;
use crate::ui::toasts::Level;
use rpack_core::{TilemapGenerationConfig, sources};
use std::path::{Path, PathBuf};

fn load_images(paths: Vec<PathBuf>) -> (Vec<AppImageData>, Vec<String>) {
    let mut images = Vec::new();
    let mut failed = Vec::new();
    for path in paths {
        match AppImageData::from_file(&path) {
            Some(image) => images.push(image),
            None => failed.push(path.display().to_string()),
        }
    }
    (images, failed)
}

fn add_paths(tx: &EventSender, paths: Vec<PathBuf>) {
    let (images, failed) = load_images(paths);
    report_failed(tx, &failed);
    if !images.is_empty() {
        tx.send(AppEvent::Images(images));
    }
}

fn is_config(path: &Path) -> bool {
    path.to_string_lossy().ends_with(".rpack_gen.json")
}

/// Reads a config and its images. The project only changes once all of that worked.
pub fn open_config(tx: EventSender, path: PathBuf) {
    jobs::spawn(move || {
        let path_text = path.to_string_lossy().to_string();
        let config = match TilemapGenerationConfig::read_from_file(&path) {
            Ok(config) => config,
            Err(error) => {
                tx.send(AppEvent::ConfigUnreadable {
                    path: path_text,
                    error: error.to_string(),
                });
                return;
            }
        };
        let paths = match config.try_file_paths() {
            Ok(paths) => paths,
            Err(error) => {
                tx.notify(
                    Level::Error,
                    format!("Could not find the images of {path_text}: {error}"),
                );
                return;
            }
        };
        let (images, failed) = load_images(paths);
        report_failed(&tx, &failed);
        if !images.is_empty() {
            tx.send(AppEvent::Config {
                config,
                path: path_text,
                images,
            });
        }
    });
}

pub fn pick_images(tx: EventSender, working_dir: String) {
    jobs::spawn_async(async move {
        let Some(files) = rfd::AsyncFileDialog::new()
            .set_directory(&working_dir)
            .set_title("Add images or open config")
            .add_filter("Files", &["png", "jpg", "jpeg", "dds", "json"])
            .pick_files()
            .await
        else {
            return;
        };
        let paths: Vec<PathBuf> = files.iter().map(|f| f.path().to_path_buf()).collect();
        if let Some(config) = paths.iter().find(|path| is_config(path)) {
            if paths.len() > 1 {
                tx.notify(
                    Level::Warning,
                    "A config was selected, the other files were ignored",
                );
            }
            open_config(tx, config.clone());
            return;
        }
        add_paths(&tx, paths);
    });
}

pub fn pick_config(tx: EventSender) {
    jobs::spawn_async(async move {
        let file = rfd::AsyncFileDialog::new()
            .set_directory(".")
            .set_title("Open config")
            .add_filter("rpack config", &["json"])
            .pick_file()
            .await;
        if let Some(file) = file {
            open_config(tx, file.path().to_path_buf());
        }
    });
}

pub fn handle_dropped(tx: EventSender, dropped: Vec<egui::DroppedFileHandle>) {
    let paths: Vec<PathBuf> = dropped.iter().map(|f| f.path().to_path_buf()).collect();
    if let Some(config) = paths.iter().find(|path| is_config(path)) {
        open_config(tx, config.clone());
        return;
    }
    jobs::spawn(move || add_paths(&tx, sources::expand_directories(paths)));
}

/// Saves the current inputs and settings as a `.rpack_gen.json` config.
pub fn save_config(tx: EventSender, config: TilemapGenerationConfig, image_paths: Vec<String>) {
    let file_name = format!("{}.rpack_gen.json", config.output_path);
    jobs::spawn_async(async move {
        let Some(file) = rfd::AsyncFileDialog::new()
            .set_directory(".")
            .add_filter("rpack config", &["json"])
            .set_file_name(file_name)
            .save_file()
            .await
        else {
            return;
        };
        let json = match config.to_project_json(&image_paths, file.path().parent()) {
            Ok(json) => json,
            Err(error) => {
                tx.notify(Level::Error, format!("Failed to save config: {error}"));
                return;
            }
        };
        match file.write(json.as_bytes()).await {
            Ok(()) => {
                tx.notify(Level::Info, format!("Saved {}", file.file_name()));
                tx.send(AppEvent::AddRecent(
                    file.path().to_string_lossy().to_string(),
                ));
            }
            Err(error) => tx.notify(Level::Error, format!("Failed to save config: {error}")),
        }
    });
}
