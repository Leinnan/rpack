//! Keyboard shortcut dispatch, driven by the menu table.
use crate::menu;

pub fn handle(ctx: &egui::Context, native_menu: bool) {
    let typing = ctx.egui_wants_keyboard_input();
    ctx.input_mut(|input| {
        for (shortcut, command) in menu::shortcuts() {
            if native_menu && menu::is_native_key_equivalent(shortcut, command) {
                continue;
            }
            if (!typing || command.works_while_typing()) && input.consume_shortcut(shortcut) {
                command.schedule();
            }
        }
    });
}
