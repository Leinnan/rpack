#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use image::DynamicImage;
use texture_packer::importer::ImageImporter;

use crate::app::AppImageData;

#[cfg(not(target_arch = "wasm32"))]
pub const IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "dds"];

pub trait DroppedFileHelper {
    fn file_path(&self) -> String;
    fn create_image<P>(&self, prefix: P) -> Option<AppImageData>
    where
        P: AsRef<str>,
    {
        let path = self.file_path();
        let base_id = rpack_cli::packing::frame_key(&path);

        let id = base_id
            .strip_prefix(prefix.as_ref())
            .unwrap_or(&base_id)
            .to_owned();

        Some(AppImageData::new(path, id, self.dynamic_image()?))
    }
    fn dynamic_image(&self) -> Option<DynamicImage>;
}

impl DroppedFileHelper for (Vec<u8>, String) {
    fn file_path(&self) -> String {
        self.1.clone()
    }

    fn dynamic_image(&self) -> Option<DynamicImage> {
        ImageImporter::import_from_memory(&self.0).ok()
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
