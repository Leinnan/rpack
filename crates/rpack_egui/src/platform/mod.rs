//! Target-specific file handling behind one interface.
pub mod jobs;
#[cfg(target_os = "macos")]
pub mod macos_menu;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;
#[cfg(target_arch = "wasm32")]
pub use web::*;

use crate::events::EventSender;
use crate::ui::toasts::Level;
use std::sync::Arc;

pub struct SaveRequest {
    pub file_name: String,
    pub filter: Option<(&'static str, &'static [&'static str])>,
    pub bytes: Arc<[u8]>,
    /// What is being saved, for the error message.
    pub what: &'static str,
}

/// Asks where to save and writes the file. On the web this is a browser download.
pub fn save_file(tx: EventSender, request: SaveRequest) {
    jobs::spawn_async(async move {
        let mut dialog = rfd::AsyncFileDialog::new()
            .set_directory(".")
            .set_file_name(request.file_name);
        if let Some((name, extensions)) = request.filter {
            dialog = dialog.add_filter(name, extensions);
        }
        let Some(file) = dialog.save_file().await else {
            return;
        };
        match file.write(&request.bytes).await {
            Ok(()) => tx.notify(Level::Info, format!("Saved {}", file.file_name())),
            Err(error) => tx.notify(
                Level::Error,
                format!("Failed to save {}: {error}", request.what),
            ),
        }
    });
}

fn short_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn report_failed(tx: &EventSender, failed: &[String]) {
    if failed.is_empty() {
        return;
    }
    let names: Vec<_> = failed.iter().take(3).map(|name| short_name(name)).collect();
    let more = failed.len().saturating_sub(names.len());
    let mut text = format!("Could not read {}", names.join(", "));
    if more > 0 {
        text.push_str(&format!(" and {more} more"));
    }
    tx.notify(Level::Error, text);
}

#[cfg(test)]
mod tests {
    use super::short_name;

    #[test]
    fn short_name_handles_both_separators() {
        assert_eq!(short_name("a/b/c.png"), "c.png");
        assert_eq!(short_name("a\\b\\c.png"), "c.png");
        assert_eq!(short_name("c.png"), "c.png");
    }
}
