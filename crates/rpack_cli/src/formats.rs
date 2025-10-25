use std::{ffi::OsStr, fmt::Display, path::Path};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Copy, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    all(feature = "cli", not(target_arch = "wasm32")),
    derive(clap::ValueEnum)
)]
pub enum SaveImageFormat {
    #[default]
    Png,
    Dds,
    Basis,
}

impl SaveImageFormat {
    /// Try to gets file extension from a path
    pub fn from_path(v: impl AsRef<Path>) -> Option<Self> {
        let path = v.as_ref();
        let extension = path.extension().and_then(OsStr::to_str)?;
        match extension {
            "png" => Some(SaveImageFormat::Png),
            "dds" => Some(SaveImageFormat::Dds),
            "basis" => Some(SaveImageFormat::Basis),
            _ => None,
        }
    }
}

impl Display for SaveImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveImageFormat::Png => f.write_str(".png"),
            SaveImageFormat::Dds => f.write_str(".dds"),
            #[cfg(feature = "basis")]
            SaveImageFormat::Basis => f.write_str(".basis"),
        }
    }
}
