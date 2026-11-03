use super::{jobs, report_failed};
use crate::events::{AppEvent, EventSender};
use crate::state::AppImageData;

pub fn pick_images(tx: EventSender, working_dir: String) {
    jobs::spawn_async(async move {
        let Some(files) = rfd::AsyncFileDialog::new()
            .set_directory(&working_dir)
            .set_title("Add images")
            .add_filter("Files", &["png", "jpg", "jpeg", "dds"])
            .pick_files()
            .await
        else {
            return;
        };
        let mut images = Vec::new();
        let mut failed = Vec::new();
        for file in files {
            let name = file.file_name();
            match AppImageData::from_bytes(&file.read().await, name.clone()) {
                Some(image) => images.push(image),
                None => failed.push(name),
            }
        }
        report_failed(&tx, &failed);
        if !images.is_empty() {
            tx.send(AppEvent::Images(images));
        }
    });
}

pub fn handle_dropped(tx: EventSender, dropped: Vec<egui::DroppedFileHandle>) {
    for file in dropped {
        let name = file.path().to_string_lossy().to_string();
        let tx = tx.clone();
        jobs::spawn_async(async move {
            let image = match file.bytes_async().await {
                Ok(bytes) => AppImageData::from_bytes(&bytes, name.clone()),
                Err(_) => None,
            };
            match image {
                Some(image) => tx.send(AppEvent::Images(vec![image])),
                None => report_failed(&tx, &[name]),
            }
        });
    }
}
