use std::path::Path;

use egui::DroppedFile;
use image::DynamicImage;
use rpack_cli::ImageFile;
use texture_packer::importer::ImageImporter;

pub fn get_common_prefix(paths: &[DroppedFile]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    let full_name = paths[0].file_path();
    let path = Path::new(&full_name)
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();

    let mut prefix = full_name.strip_suffix(&path).unwrap_or_default().to_owned();

    for s in paths.iter().skip(1) {
        let s = s.file_path();
        while !(s.starts_with(&prefix) || prefix.is_empty()) {
            prefix.pop();
        }
    }

    prefix
}

pub trait DroppedFileHelper {
    fn file_path(&self) -> String;
    fn create_image<P>(&self, prefix: P) -> Option<(String, ImageFile)>
    where
        P: AsRef<str>;
    fn dynamic_image(&self) -> Option<DynamicImage>;
}

impl DroppedFileHelper for DroppedFile {
    fn file_path(&self) -> String {
        let id;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = self.path.as_ref().unwrap().clone();
            id = path.to_str().unwrap().to_owned();
        }
        #[cfg(target_arch = "wasm32")]
        {
            id = self.name.clone();
        }
        id.replace(".png", "").replace("\\", "/")
    }
    fn create_image<P>(&self, prefix: P) -> Option<(String, ImageFile)>
    where
        P: AsRef<str>,
    {
        let path = self.file_path();
        let base_id = path.replace(".png", "");

        let id = base_id
            .strip_prefix(prefix.as_ref())
            .unwrap_or(&base_id)
            .to_owned();

        let image: DynamicImage = self.dynamic_image()?;
        Some((path, ImageFile { id, image }))
    }

    fn dynamic_image(&self) -> Option<DynamicImage> {
        #[cfg(target_arch = "wasm32")]
        {
            let bytes = self.bytes.as_ref().clone()?;

            if let Ok(r) = ImageImporter::import_from_memory(bytes) {
                Some(r.into())
            } else {
                None
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = self.path.as_ref()?;

            if let Ok(r) = ImageImporter::import_from_file(path) {
                Some(r)
            } else {
                None
            }
        }
    }
}
