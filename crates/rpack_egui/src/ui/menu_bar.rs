//! Draws the menu table with egui.
use super::{icon_label, toolbar};
use crate::menu::{self, Entry, Item};
use crate::state::AppState;
use egui::{Button, Slider, Ui};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    egui::MenuBar::new().ui(ui, |ui| {
        if state.ui.native_chrome {
            ui.add_space(toolbar::leading_inset(ui, state));
        }
        for menu in menu::menus() {
            ui.menu_button(menu.title, |ui| {
                for entry in &menu.entries {
                    show_entry(ui, state, entry);
                }
            });
        }
    });
}

fn show_entry(ui: &mut Ui, state: &mut AppState, entry: &Entry) {
    match entry {
        Entry::Item(item) => show_item(ui, state, item),
        Entry::Toggle {
            label,
            command,
            checked,
        } => {
            let mut value = checked(state);
            if ui.checkbox(&mut value, *label).changed() {
                command.schedule();
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        Entry::Recents => {
            use crate::commands::Command;
            use lucide_icons::Icon;
            ui.add_enabled_ui(!state.ui.settings.recents.is_empty(), |ui| {
                ui.menu_button(icon_label(Icon::FolderClock, menu::OPEN_RECENT), |ui| {
                    for (index, path) in state.ui.settings.recents.iter().enumerate() {
                        if ui.button(path).clicked() {
                            Command::OpenRecent(index).schedule();
                            ui.close();
                        }
                    }
                });
            });
        }
        Entry::ScaleSlider => {
            ui.label(menu::INTERFACE_SCALE);
            ui.add(Slider::new(&mut state.ui.settings.ui_scale, 1.0..=2.0));
        }
        Entry::Separator => {
            ui.separator();
        }
    }
}

fn show_item(ui: &mut Ui, state: &AppState, item: &Item) {
    let mut button = Button::new(icon_label(item.icon, item.label));
    if let Some(shortcut) = item.shortcuts.first() {
        button = button.shortcut_text(ui.ctx().format_shortcut(shortcut));
    }
    if ui
        .add_enabled(item.command.enabled(state), button)
        .clicked()
    {
        item.command.schedule();
        ui.close();
    }
}
