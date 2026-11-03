//! The native macOS menu bar, built from the menu table with muda.
//!
//! Clicks and key equivalents schedule commands like any other UI. Shortcuts without
//! modifiers, such as Delete, stay with egui so they never take keys from a text field.
//!
//! Key equivalents reach AppKit before egui. While a text field has focus, the Edit
//! commands are turned back into the egui events the field expects.
use crate::commands::Command;
use crate::menu::{self, Entry};
use crate::state::AppState;
use egui::KeyboardShortcut;
use muda::{
    CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu,
    accelerator::{Accelerator, Code, Modifiers},
};
use std::cell::RefCell;
use std::collections::HashMap;

const RECENT_ID: &str = "recent:";
const SCALE_PERCENTS: [u16; 5] = [100, 125, 150, 175, 200];

// Menu events arrive on the main thread, the same one that runs the frames.
thread_local! {
    static TEXT_EVENTS: RefCell<Vec<egui::Event>> = const { RefCell::new(Vec::new()) };
}

/// Events for the focused text field, to feed into the next frame's input.
pub fn drain_text_events() -> Vec<egui::Event> {
    TEXT_EVENTS.with(|events| std::mem::take(&mut *events.borrow_mut()))
}

/// The events a text field expects for `command`, if it is a text editing one.
fn text_events_for(command: Command) -> Option<Vec<egui::Event>> {
    let key = |key, modifiers| {
        [true, false]
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            })
            .to_vec()
    };
    let command_key = egui::Modifiers::COMMAND;
    match command {
        Command::SelectAll => Some(key(egui::Key::A, command_key)),
        Command::Undo => Some(key(egui::Key::Z, command_key)),
        Command::Redo => Some(key(egui::Key::Z, command_key | egui::Modifiers::SHIFT)),
        _ => None,
    }
}

/// A check item and the state it shows.
type Toggle = (fn(&AppState) -> bool, CheckMenuItem);

/// What building the menus collects.
#[derive(Default)]
struct Collect {
    ids: HashMap<MenuId, Command>,
    items: Vec<(Command, MenuItem)>,
    toggles: Vec<Toggle>,
    scales: Vec<(u16, CheckMenuItem)>,
    recent: Option<Submenu>,
}

/// Keeps the native menu alive for the lifetime of the app.
pub struct NativeMenuBar {
    _menu: Menu,
    items: Vec<(Command, MenuItem)>,
    toggles: Vec<Toggle>,
    scales: Vec<(u16, CheckMenuItem)>,
    recent: Option<Submenu>,
    recents_shown: Option<Vec<String>>,
}

impl NativeMenuBar {
    /// Must run on the main thread after `NSApp` exists, so from the eframe app creator.
    pub fn install(ctx: &egui::Context) -> Result<Self, muda::Error> {
        let menu = Menu::new();
        let mut collect = Collect::default();

        let app_menu = Submenu::new("rPack", true);
        app_menu.append_items(&[
            // Our own sheet rather than AppKit's panel.
            &command_item(&mut collect.ids, "About rPack", Command::ShowAbout, None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::services(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(None),
            &PredefinedMenuItem::hide_others(None),
            &PredefinedMenuItem::show_all(None),
            &PredefinedMenuItem::separator(),
            // Our own Quit, so eframe shuts down cleanly and saves the settings.
            &command_item(
                &mut collect.ids,
                "Quit rPack",
                Command::Quit,
                Some(KeyboardShortcut::new(
                    egui::Modifiers::COMMAND,
                    egui::Key::Q,
                )),
            ),
        ])?;
        menu.append(&app_menu)?;

        for top in menu::menus() {
            let submenu = Submenu::new(top.title, true);
            for entry in &top.entries {
                append_entry(&submenu, entry, &mut collect)?;
            }
            // About lives in the app menu on macOS, which can leave Help empty.
            if !submenu.items().is_empty() {
                menu.append(&submenu)?;
            }
        }

        let window = Submenu::new("Window", true);
        window.append_items(&[
            &PredefinedMenuItem::minimize(None),
            &PredefinedMenuItem::maximize(Some("Zoom")),
            &PredefinedMenuItem::fullscreen(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::close_window(None),
        ])?;
        menu.append(&window)?;
        window.set_as_windows_menu_for_nsapp();
        menu.init_for_nsapp();

        let Collect {
            ids,
            items,
            toggles,
            scales,
            recent,
        } = collect;
        let ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let command = ids.get(event.id()).copied().or_else(|| {
                event
                    .id()
                    .as_ref()
                    .strip_prefix(RECENT_ID)
                    .and_then(|index| index.parse().ok())
                    .map(Command::OpenRecent)
            });
            let Some(command) = command else {
                return;
            };
            let text_events = ctx
                .egui_wants_keyboard_input()
                .then(|| text_events_for(command))
                .flatten();
            match text_events {
                Some(text_events) => {
                    TEXT_EVENTS.with(|events| events.borrow_mut().extend(text_events));
                }
                None => command.schedule(),
            }
            ctx.request_repaint();
        }));

        Ok(Self {
            _menu: menu,
            items,
            toggles,
            scales,
            recent,
            recents_shown: None,
        })
    }

    /// Mirrors the state into the menu: greyed out items, check marks and recents.
    pub fn sync(&mut self, state: &AppState) {
        for (command, item) in &self.items {
            let enabled = command.enabled(state);
            if item.is_enabled() != enabled {
                item.set_enabled(enabled);
            }
        }
        for (checked, item) in &self.toggles {
            set_checked(item, checked(state));
        }
        let scale = (state.ui.settings.ui_scale * 100.0).round() as u16;
        for (percent, item) in &self.scales {
            set_checked(item, *percent == scale);
        }
        let recents = &state.ui.settings.recents;
        if self.recents_shown.as_ref() != Some(recents) {
            self.show_recents(recents);
            self.recents_shown = Some(recents.clone());
        }
    }

    fn show_recents(&self, recents: &[String]) {
        let Some(submenu) = &self.recent else {
            return;
        };
        while submenu.remove_at(0).is_some() {}
        submenu.set_enabled(!recents.is_empty());
        for (index, path) in recents.iter().enumerate() {
            let item = MenuItem::with_id(format!("{RECENT_ID}{index}"), path, true, None);
            if let Err(error) = submenu.append(&item) {
                log::warn!("Could not update Open recent: {error}");
            }
        }
    }
}

fn set_checked(item: &CheckMenuItem, checked: bool) {
    if item.is_checked() != checked {
        item.set_checked(checked);
    }
}

fn append_entry(
    submenu: &Submenu,
    entry: &Entry,
    collect: &mut Collect,
) -> Result<(), muda::Error> {
    match entry {
        Entry::Item(item) if item.command == Command::ShowAbout => {}
        Entry::Item(item) => {
            let shortcut = item.shortcuts.first().copied();
            let native = command_item(&mut collect.ids, item.label, item.command, shortcut);
            submenu.append(&native)?;
            collect.items.push((item.command, native));
        }
        Entry::Toggle {
            label,
            command,
            checked,
        } => {
            let item = check_item(&mut collect.ids, label, *command);
            submenu.append(&item)?;
            collect.toggles.push((*checked, item));
        }
        Entry::Recents => {
            let recent = Submenu::new(menu::OPEN_RECENT, false);
            submenu.append(&recent)?;
            collect.recent = Some(recent);
        }
        // A menu item can't host a slider.
        Entry::ScaleSlider => {
            let scales = Submenu::new(menu::INTERFACE_SCALE, true);
            for percent in SCALE_PERCENTS {
                let item = check_item(
                    &mut collect.ids,
                    &format!("{percent}%"),
                    Command::SetInterfaceScale(percent),
                );
                scales.append(&item)?;
                collect.scales.push((percent, item));
            }
            submenu.append(&scales)?;
        }
        Entry::Separator => submenu.append(&PredefinedMenuItem::separator())?,
    }
    Ok(())
}

fn command_id(ids: &mut HashMap<MenuId, Command>, command: Command) -> MenuId {
    let id = MenuId::new(format!("{command:?}"));
    ids.insert(id.clone(), command);
    id
}

fn command_item(
    ids: &mut HashMap<MenuId, Command>,
    label: &str,
    command: Command,
    shortcut: Option<KeyboardShortcut>,
) -> MenuItem {
    MenuItem::with_id(
        command_id(ids, command),
        label,
        true,
        shortcut.and_then(accelerator),
    )
}

fn check_item(ids: &mut HashMap<MenuId, Command>, label: &str, command: Command) -> CheckMenuItem {
    CheckMenuItem::with_id(command_id(ids, command), label, true, false, None)
}

/// The native key equivalent. `None` for shortcuts without modifiers, which egui keeps.
fn accelerator(shortcut: KeyboardShortcut) -> Option<Accelerator> {
    let held = shortcut.modifiers;
    let mut modifiers = Modifiers::empty();
    if held.command || held.mac_cmd {
        modifiers |= Modifiers::SUPER;
    }
    if held.ctrl {
        modifiers |= Modifiers::CONTROL;
    }
    if held.shift {
        modifiers |= Modifiers::SHIFT;
    }
    if held.alt {
        modifiers |= Modifiers::ALT;
    }
    if modifiers.is_empty() {
        return None;
    }
    Some(Accelerator::new(
        Some(modifiers),
        key_code(shortcut.logical_key)?,
    ))
}

/// Only the keys in use. A test fails when the menu table needs another one.
fn key_code(key: egui::Key) -> Option<Code> {
    Some(match key {
        egui::Key::A => Code::KeyA,
        egui::Key::O => Code::KeyO,
        egui::Key::Q => Code::KeyQ,
        egui::Key::S => Code::KeyS,
        egui::Key::Z => Code::KeyZ,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};

    #[test]
    fn menu_shortcuts_with_modifiers_become_key_equivalents() {
        for (shortcut, command) in menu::shortcuts() {
            if menu::is_native_key_equivalent(shortcut, command) {
                assert!(accelerator(*shortcut).is_some(), "{command:?} has no key");
            }
        }
    }

    #[test]
    fn only_edit_commands_go_to_the_text_field() {
        assert!(text_events_for(Command::SaveAtlas).is_none());
        assert!(text_events_for(Command::RemoveSelected).is_none());
        assert!(text_events_for(Command::SelectAll).is_some());
    }

    #[test]
    fn select_all_reaches_the_focused_field() {
        let mut harness = Harness::new_ui_state(
            |ui, text: &mut String| {
                let label = ui.label("Name");
                ui.text_edit_singleline(text).labelled_by(label.id);
            },
            "hello".to_owned(),
        );
        harness
            .get_by_role(egui::accesskit::Role::TextInput)
            .focus();
        harness.run();
        for event in text_events_for(Command::SelectAll).unwrap_or_default() {
            harness.event(event);
        }
        harness.run();
        harness.event(egui::Event::Cut);
        harness.run();
        assert_eq!(harness.state(), "");
    }
}
