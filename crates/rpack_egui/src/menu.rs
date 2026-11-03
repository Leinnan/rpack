//! The menu table: drives the menu bar, shortcut dispatch and tooltips.
use crate::commands::Command;
use crate::state::AppState;
use egui::{Key, KeyboardShortcut, Modifiers};
use lucide_icons::Icon;
use std::sync::LazyLock;

const SHIFT_COMMAND: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);

#[cfg(not(target_arch = "wasm32"))]
pub const OPEN_RECENT: &str = "Open recent";
pub const INTERFACE_SCALE: &str = "Interface scale";

pub struct Item {
    pub label: &'static str,
    pub icon: Icon,
    pub command: Command,
    /// Shortcuts of the command. The first one is shown in menus and tooltips.
    pub shortcuts: &'static [KeyboardShortcut],
}

pub enum Entry {
    Item(Item),
    Toggle {
        label: &'static str,
        command: Command,
        checked: fn(&AppState) -> bool,
    },
    #[cfg(not(target_arch = "wasm32"))]
    Recents,
    ScaleSlider,
    Separator,
}

pub struct Menu {
    pub title: &'static str,
    pub entries: Vec<Entry>,
}

const fn item(
    label: &'static str,
    icon: Icon,
    command: Command,
    shortcuts: &'static [KeyboardShortcut],
) -> Entry {
    Entry::Item(Item {
        label,
        icon,
        command,
        shortcuts,
    })
}

const OPEN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
const SAVE_ATLAS: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
const SAVE_JSON: KeyboardShortcut = KeyboardShortcut::new(SHIFT_COMMAND, Key::S);
const UNDO: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Z);
const REDO: KeyboardShortcut = KeyboardShortcut::new(SHIFT_COMMAND, Key::Z);
const REDO_ALT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Y);
const SELECT_ALL: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::A);
const REMOVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);
const REMOVE_ALT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::Backspace);

static MENUS: LazyLock<Vec<Menu>> = LazyLock::new(|| {
    vec![
        Menu {
            title: "File",
            entries: vec![
                item(
                    "Add images...",
                    Icon::ImagePlus,
                    Command::AddImages,
                    &[OPEN],
                ),
                item(
                    "Save atlas image",
                    Icon::Save,
                    Command::SaveAtlas,
                    &[SAVE_ATLAS],
                ),
                item(
                    "Save atlas json",
                    Icon::Braces,
                    Command::SaveJson,
                    &[SAVE_JSON],
                ),
                #[cfg(not(target_arch = "wasm32"))]
                item("Save config...", Icon::FileCog, Command::SaveConfig, &[]),
                Entry::Separator,
                #[cfg(not(target_arch = "wasm32"))]
                item("Open config...", Icon::FolderOpen, Command::OpenConfig, &[]),
                #[cfg(not(target_arch = "wasm32"))]
                Entry::Recents,
            ],
        },
        Menu {
            title: "Edit",
            entries: vec![
                item("Undo", Icon::Undo2, Command::Undo, &[UNDO]),
                item("Redo", Icon::Redo2, Command::Redo, &[REDO, REDO_ALT]),
                Entry::Separator,
                item(
                    "Select all images",
                    Icon::SquareDashed,
                    Command::SelectAll,
                    &[SELECT_ALL],
                ),
                item(
                    "Remove selected",
                    Icon::Trash2,
                    Command::RemoveSelected,
                    &[REMOVE, REMOVE_ALT],
                ),
                item("Remove all images", Icon::ListX, Command::ClearAll, &[]),
            ],
        },
        Menu {
            title: "View",
            entries: vec![
                Entry::Toggle {
                    label: "Show atlas JSON",
                    command: Command::ToggleJson,
                    checked: |state| state.ui.settings.display_json,
                },
                Entry::Toggle {
                    label: "Show sprite outlines",
                    command: Command::ToggleOutlines,
                    checked: |state| state.ui.settings.show_outlines,
                },
                Entry::Toggle {
                    label: "Show sprites panel",
                    command: Command::ToggleSprites,
                    checked: |state| state.ui.settings.show_sprites,
                },
                Entry::Toggle {
                    label: "Show inspector",
                    command: Command::ToggleInspector,
                    checked: |state| state.ui.inspector_visible(),
                },
                Entry::Separator,
                item("Fit preview", Icon::Maximize, Command::FitPreview, &[]),
                item("Actual size", Icon::ZoomIn, Command::ActualSize, &[]),
                Entry::Separator,
                Entry::ScaleSlider,
            ],
        },
        Menu {
            title: "Help",
            entries: vec![item("About rPack", Icon::Info, Command::ShowAbout, &[])],
        },
    ]
});

pub fn menus() -> &'static [Menu] {
    &MENUS
}

fn items() -> impl Iterator<Item = &'static Item> {
    menus()
        .iter()
        .flat_map(|menu| &menu.entries)
        .filter_map(|entry| match entry {
            Entry::Item(item) => Some(item),
            _ => None,
        })
}

/// The shortcut shown for `command`, if it has one.
pub fn shortcut_for(command: Command) -> Option<&'static KeyboardShortcut> {
    items()
        .find(|item| item.command == command)
        .and_then(|item| item.shortcuts.first())
}

/// All shortcuts, those with more modifiers first. `consume_shortcut` ignores extra
/// Shift and Alt, so Cmd+Z would otherwise also match Cmd+Shift+Z.
pub fn shortcuts() -> Vec<(&'static KeyboardShortcut, Command)> {
    let mut shortcuts: Vec<_> = items()
        .flat_map(|item| item.shortcuts.iter().map(|s| (s, item.command)))
        .collect();
    shortcuts.sort_by_key(|(s, _)| std::cmp::Reverse(modifier_count(s.modifiers)));
    shortcuts
}

/// Whether a native menu handles this shortcut itself, as a key equivalent of its item.
pub fn is_native_key_equivalent(shortcut: &KeyboardShortcut, command: Command) -> bool {
    shortcut.modifiers != Modifiers::NONE && shortcut_for(command) == Some(shortcut)
}

fn modifier_count(modifiers: Modifiers) -> u32 {
    u32::from(modifiers.alt)
        + u32::from(modifiers.ctrl)
        + u32::from(modifiers.shift)
        + u32::from(modifiers.mac_cmd)
        + u32::from(modifiers.command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn shortcuts_are_unique() {
        let mut seen = HashSet::new();
        for (shortcut, command) in shortcuts() {
            assert!(
                seen.insert((shortcut.modifiers, shortcut.logical_key)),
                "{command:?} reuses a shortcut"
            );
        }
    }

    #[test]
    fn longer_shortcuts_are_checked_first() {
        let order: Vec<_> = shortcuts().into_iter().map(|(_, c)| c).collect();
        let position = |c| order.iter().position(|o| *o == c).unwrap();
        assert!(position(Command::Redo) < position(Command::Undo));
        assert!(position(Command::SaveJson) < position(Command::SaveAtlas));
    }
}
