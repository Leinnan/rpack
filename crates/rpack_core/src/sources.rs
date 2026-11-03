//! Finding source images on disk and writing their paths into configs.
use std::path::{Path, PathBuf};

/// File extensions offered by file dialogs and picked up from dropped folders.
pub const IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "dds"];

fn has_image_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| IMAGE_EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// Expands directories (one level deep) into the image files they contain.
pub fn expand_directories(paths: Vec<PathBuf>) -> Vec<PathBuf> {
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

/// Escapes a path so a glob pattern matches exactly that path.
pub fn glob_escape(path: &str) -> String {
    path.replace('\\', "/")
        .replace('[', "[[]")
        .replace('*', "[*]")
        .replace('?', "[?]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_only_images_one_level_deep_in_order() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../bevy_rpack");
        let files = expand_directories(vec![root.join("assets")]);
        assert!(files.iter().all(|path| has_image_extension(path)));
        assert!(files.ends_with(&[root.join("assets/tilemap.png")]));
        let tiles = expand_directories(vec![root.join("tiles")]);
        assert!(tiles.is_empty(), "does not recurse: {tiles:?}");
    }

    #[test]
    fn keeps_plain_files_even_with_other_extensions() {
        let file = PathBuf::from("missing/notes.txt");
        assert_eq!(expand_directories(vec![file.clone()]), [file]);
    }

    #[test]
    fn escapes_glob_characters_and_separators() {
        assert_eq!(glob_escape("a\\b[1]*?.png"), "a/b[[]1][*][?].png");
    }
}
