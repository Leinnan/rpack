//! Preferences kept between runs.
use serde::{Deserialize, Serialize};

const STORAGE_KEY: &str = "settings";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub display_json: bool,
    pub show_outlines: bool,
    pub show_sprites: bool,
    pub show_inspector: bool,
    /// Scale of the whole interface.
    pub ui_scale: f32,
    /// Recently opened configs, newest first.
    pub recents: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            display_json: true,
            show_outlines: false,
            show_sprites: true,
            show_inspector: true,
            ui_scale: 1.0,
            recents: Vec::new(),
        }
    }
}

impl Settings {
    /// Before 0.7 only the recent list was stored under `APP_KEY`. It is carried over;
    /// the old `view_settings` are not, as their scale compensated for the old styling.
    pub fn load(storage: Option<&dyn eframe::Storage>) -> Self {
        let Some(storage) = storage else {
            return Self::default();
        };
        eframe::get_value(storage, STORAGE_KEY).unwrap_or_else(|| Self {
            recents: eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default(),
            ..Self::default()
        })
    }

    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, STORAGE_KEY, self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::Storage;
    use std::collections::HashMap;

    #[derive(Default)]
    struct MemoryStorage(HashMap<String, String>);

    impl eframe::Storage for MemoryStorage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.to_owned(), value);
        }
        fn remove_string(&mut self, key: &str) {
            self.0.remove(key);
        }
        fn flush(&mut self) {}
    }

    #[test]
    fn old_recents_are_carried_over_without_the_old_scale() {
        let mut storage = MemoryStorage::default();
        eframe::set_value(&mut storage, eframe::APP_KEY, &vec!["a.rpack_gen.json"]);
        storage.set_string("view_settings", r#"{"ui_scale":1.4}"#.to_owned());
        let settings = Settings::load(Some(&storage));
        assert_eq!(settings.recents, ["a.rpack_gen.json"]);
        assert_eq!(settings.ui_scale, 1.0);
    }

    #[test]
    fn unknown_and_missing_fields_keep_defaults() {
        let mut storage = MemoryStorage::default();
        storage.set_string(STORAGE_KEY, r#"(show_outlines:true,removed:1)"#.to_owned());
        let settings = Settings::load(Some(&storage));
        assert!(settings.show_outlines);
        assert!(settings.display_json);
    }
}
