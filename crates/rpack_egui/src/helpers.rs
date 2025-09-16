#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use egui::DroppedFile;
use image::DynamicImage;
use rpack_cli::ImageFile;
use texture_packer::importer::ImageImporter;

use crate::app::AppImageData;

pub trait DroppedFileHelper {
    fn file_path(&self) -> String;
    fn create_image<P>(&self, prefix: P) -> Option<AppImageData>
    where
        P: AsRef<str>,
    {
        let path = self.file_path();
        let base_id = id_from_path(&path);

        let id = base_id
            .strip_prefix(prefix.as_ref())
            .unwrap_or(&base_id)
            .to_owned();

        let image: DynamicImage = self.dynamic_image()?;
        Some(AppImageData {
            width: image.width(),
            height: image.height(),
            data: ImageFile { id, image },
            path,
        })
    }
    fn dynamic_image(&self) -> Option<DynamicImage>;
}
#[cfg(not(target_arch = "wasm32"))]
impl DroppedFileHelper for std::fs::DirEntry {
    fn file_path(&self) -> String {
        self.path().to_string_lossy().to_string()
    }

    fn dynamic_image(&self) -> Option<DynamicImage> {
        ImageImporter::import_from_file(&self.path()).ok()
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl DroppedFileHelper for PathBuf {
    fn file_path(&self) -> String {
        self.display().to_string()
    }

    fn dynamic_image(&self) -> Option<DynamicImage> {
        ImageImporter::import_from_file(self.as_path()).ok()
    }
}

pub fn id_from_path(path: &str) -> String {
    match path.rfind('.') {
        Some(index) => path[..index].to_string(),
        None => path.to_string(),
    }
    .replace("\\", "/")
}

impl DroppedFileHelper for DroppedFile {
    fn file_path(&self) -> String {
        match self.path.as_ref() {
            Some(path) => path.to_string_lossy().to_string(),
            None => self.name.clone(),
        }
    }

    fn dynamic_image(&self) -> Option<DynamicImage> {
        #[cfg(target_arch = "wasm32")]
        {
            let bytes = self.bytes.as_ref().clone()?;

            ImageImporter::import_from_memory(bytes)
                .ok()
                .map(|r| r.into())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = self.path.as_ref()?;

            ImageImporter::import_from_file(path).ok()
        }
    }
}
