//! Finding source images on disk and writing their paths into configs.
use std::path::{Path, PathBuf};

/// File extensions offered by file dialogs and picked up from dropped folders.
pub const IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "dds"];

fn has_image_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| IMAGE_EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// Expands directories, including their subdirectories, into the image files they contain.
pub fn expand_directories(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            collect_images(&path, &mut files);
        } else {
            files.push(path);
        }
    }
    files
}

/// Symlinked directories are skipped, as a link to a parent would never end.
fn collect_images(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(dir) = dir.read_dir() else {
        return;
    };
    let mut entries: Vec<_> = dir.flatten().collect();
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            collect_images(&path, files);
        } else if path.is_file() && has_image_extension(&path) {
            files.push(path);
        }
    }
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
    fn expands_only_images_in_order() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../bevy_rpack");
        let files = expand_directories(vec![root.join("assets")]);
        assert!(files.iter().all(|path| has_image_extension(path)));
        assert!(files.contains(&root.join("assets/tilemap.png")));
    }

    #[test]
    fn expands_subdirectories() {
        let tiles = Path::new(env!("CARGO_MANIFEST_DIR")).join("../bevy_rpack/tiles");
        let files = expand_directories(vec![tiles.clone()]);
        assert!(files.contains(&tiles.join("agents/spaceAstronauts_004.png")));
        assert!(files.contains(&tiles.join("ship/spaceBuilding_001.png")));
        assert!(files.is_sorted());
    }

    #[cfg(unix)]
    #[test]
    fn does_not_follow_symlinked_directories() {
        let root = std::env::temp_dir().join(format!("rpack_sources_{}", std::process::id()));
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("sub/a.png"), []).unwrap();
        std::os::unix::fs::symlink(&root, root.join("sub/parent")).unwrap();
        let files = expand_directories(vec![root.clone()]);
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(files, [root.join("sub/a.png")]);
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
