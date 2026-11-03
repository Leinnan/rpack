//! The `.rpack_gen.json` generation config.
use crate::{SaveImageFormat, SpritesheetBuildConfig, packing, sources};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use texture_packer::TexturePackerConfig;

#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TilemapGenerationConfig {
    pub asset_patterns: Vec<String>,
    pub output_path: String,
    /// Image format, png by default
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub format: Option<SaveImageFormat>,
    /// Size of the tilemap texture. Default value is `2048`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub size: Option<u32>,
    /// Size of the padding between frames in pixel. Default value is `2`
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub texture_padding: Option<u32>,
    /// Size of the padding on the outer edge of the packed image in pixel. Default value is `0`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub border_padding: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub skip_serializing_metadata: Option<bool>,
    #[serde(skip)]
    pub working_dir: Option<PathBuf>,
}

impl From<&TilemapGenerationConfig> for TexturePackerConfig {
    fn from(config: &TilemapGenerationConfig) -> Self {
        packing::packer_config(config.size, config.texture_padding, config.border_padding)
    }
}
impl From<&TilemapGenerationConfig> for SpritesheetBuildConfig {
    fn from(config: &TilemapGenerationConfig) -> Self {
        SpritesheetBuildConfig {
            packer_config: config.into(),
            skip_metadata_serialization: config.skip_serializing_metadata.unwrap_or_default(),
        }
    }
}

impl TilemapGenerationConfig {
    /// Parses a config without touching the file system. `working_dir` stays unset.
    pub fn from_json(bytes: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(bytes)
    }

    /// Serializes this config with `image_paths` as its patterns. Paths inside
    /// `config_dir` become relative to it, and all of them are escaped for glob.
    pub fn to_project_json(
        &self,
        image_paths: &[String],
        config_dir: Option<&Path>,
    ) -> serde_json::Result<String> {
        let asset_patterns = image_paths
            .iter()
            .map(|path| {
                let relative = config_dir
                    .and_then(|dir| Path::new(path).strip_prefix(dir).ok())
                    .map(|p| p.to_string_lossy().to_string());
                sources::glob_escape(relative.as_deref().unwrap_or(path))
            })
            .collect();
        serde_json::to_string_pretty(&Self {
            asset_patterns,
            ..self.clone()
        })
    }
}

#[cfg(all(feature = "config_ext", not(target_arch = "wasm32")))]
impl TilemapGenerationConfig {
    pub fn read_from_file<P>(path: P) -> anyhow::Result<TilemapGenerationConfig>
    where
        P: AsRef<Path>,
    {
        let config_file = std::fs::read(path.as_ref())?;
        let mut config = Self::from_json(&config_file)?;
        config.working_dir = Path::new(path.as_ref()).parent().map(|p| p.to_path_buf());
        Ok(config)
    }

    /// Discover sorted, unique source files and propagate invalid patterns/read errors.
    pub fn try_file_paths(&self) -> anyhow::Result<Vec<PathBuf>> {
        let working_dir = self.working_dir();
        let mut paths = std::collections::BTreeSet::new();
        for pattern in &self.asset_patterns {
            let full = working_dir.join(pattern);
            for path in glob::glob(&full.to_string_lossy())? {
                let path = path?;
                if path.is_file() {
                    paths.insert(path);
                }
            }
        }
        if paths.is_empty() {
            anyhow::bail!("no source images matched");
        }
        Ok(paths.into_iter().collect())
    }

    /// Lossy form of [`Self::try_file_paths`]: any error gives an empty list.
    pub fn get_file_paths_and_prefix(&self) -> (Vec<PathBuf>, String) {
        let paths = self.try_file_paths().unwrap_or_default();
        let prefix = crate::get_common_prefix(&paths);
        (paths, prefix)
    }

    pub fn working_dir(&self) -> PathBuf {
        let dir = match &self.working_dir {
            None => std::env::current_dir().expect("msg"),
            Some(p) => {
                if p.as_os_str().is_empty() {
                    std::env::current_dir().expect("msg")
                } else {
                    p.clone()
                }
            }
        };

        std::path::absolute(dir).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_json_round_trips_with_relative_escaped_patterns() {
        let config = TilemapGenerationConfig {
            output_path: "atlas".into(),
            size: Some(512),
            ..Default::default()
        };
        let paths = vec!["/art/a[1].png".to_owned(), "/other/b.png".to_owned()];
        let json = config
            .to_project_json(&paths, Some(Path::new("/art")))
            .unwrap();
        let parsed = TilemapGenerationConfig::from_json(json.as_bytes()).unwrap();
        assert_eq!(parsed.asset_patterns, ["a[[]1].png", "/other/b.png"]);
        assert_eq!(parsed.output_path, "atlas");
        assert_eq!(parsed.size, Some(512));
    }

    #[cfg(all(feature = "config_ext", not(target_arch = "wasm32")))]
    #[test]
    fn discovery_deduplicates_and_reports_errors() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../bevy_rpack/assets/tilemap.png");
        let mut config = TilemapGenerationConfig {
            asset_patterns: vec![path.display().to_string(); 2],
            ..Default::default()
        };
        assert_eq!(config.try_file_paths().unwrap().len(), 1);
        config.asset_patterns = vec!["[".into()];
        assert!(config.try_file_paths().is_err());
        config.asset_patterns = vec!["missing-file-123.png".into()];
        assert!(config.try_file_paths().is_err());
    }
}
