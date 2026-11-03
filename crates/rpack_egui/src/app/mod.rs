//! The application shell: frame flow, applying commands and worker events.
mod build;
mod edit;
mod file;
mod keys;
mod view;

use crate::commands::{self, Command};
use crate::events::{self, AppEvent, EventInbox, EventSender};
use crate::platform;
use crate::settings::Settings;
use crate::state::{AppState, Dialog};
use crate::ui;
use crate::ui::toasts::Level;

pub struct Application {
    pub(crate) state: AppState,
    pub(crate) sender: EventSender,
    inbox: EventInbox,
    applied_scale: f32,
    window_title: String,
    #[cfg(target_os = "macos")]
    native_menu: Option<platform::macos_menu::NativeMenuBar>,
    #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
    debug_capture: Option<crate::debug_capture::DebugCapture>,
}

impl Application {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>, config_file: Option<String>) -> Self {
        Self::create(cc, config_file, true)
    }

    /// `native` (system fonts, native menu and title bar) is off in tests, which must
    /// not depend on the machine.
    pub(crate) fn create(
        cc: &eframe::CreationContext<'_>,
        config_file: Option<String>,
        native: bool,
    ) -> Self {
        let ctx = &cc.egui_ctx;
        egui_extras::install_image_loaders(ctx);
        ui::install(ctx, native);

        #[allow(unused_mut)]
        let mut settings = Settings::load(cc.storage);
        #[cfg(not(target_arch = "wasm32"))]
        settings
            .recents
            .retain(|path| std::path::Path::new(path).exists());
        ctx.set_zoom_factor(settings.ui_scale);

        let (sender, inbox) = events::channel(ctx);
        let mut state = AppState::default();
        let applied_scale = settings.ui_scale;
        state.ui.settings = settings;

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(config_file) = config_file {
            platform::open_config(sender.clone(), config_file.into());
        }
        #[cfg(target_arch = "wasm32")]
        let _ = config_file;

        #[cfg(target_os = "macos")]
        let native_menu = native
            .then(|| platform::macos_menu::NativeMenuBar::install(ctx))
            .and_then(|menu| {
                menu.inspect_err(|error| log::error!("Native menu bar failed: {error}"))
                    .ok()
            });
        #[cfg(target_os = "macos")]
        {
            state.ui.native_chrome = native;
            state.ui.native_menu = native_menu.is_some();
        }

        Self {
            state,
            sender,
            inbox,
            applied_scale,
            window_title: String::new(),
            #[cfg(target_os = "macos")]
            native_menu,
            #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
            debug_capture: crate::debug_capture::DebugCapture::from_env(),
        }
    }

    fn handle_event(&mut self, ctx: &egui::Context, event: AppEvent) {
        match event {
            AppEvent::Images(images) => self.merge_images(ctx, images),
            AppEvent::Build(generation, outcome) => self.finish_build(ctx, generation, outcome),
            AppEvent::Notify(level, text) => self.state.ui.toasts.push(ctx, level, text),
            #[cfg(not(target_arch = "wasm32"))]
            AppEvent::Config {
                config,
                path,
                images,
            } => {
                self.add_recent(path);
                self.replace_project(ctx, config, images);
            }
            #[cfg(not(target_arch = "wasm32"))]
            AppEvent::ConfigUnreadable { path, error } => {
                self.state.ui.settings.recents.retain(|p| *p != path);
                self.state.ui.toasts.push(
                    ctx,
                    Level::Error,
                    format!("Could not open {path}: {error}"),
                );
            }
            #[cfg(not(target_arch = "wasm32"))]
            AppEvent::AddRecent(path) => self.add_recent(path),
        }
    }

    fn run_commands(&mut self, ctx: &egui::Context) {
        for command in commands::drain() {
            if command.enabled(&self.state) {
                self.apply(ctx, command);
            }
        }
    }

    fn apply(&mut self, ctx: &egui::Context, command: Command) {
        match command {
            Command::AddImages => self.add_images(),
            #[cfg(not(target_arch = "wasm32"))]
            Command::OpenConfig => self.pick_config(),
            Command::SaveAtlas => self.save_atlas(),
            Command::SaveJson => self.save_json(),
            #[cfg(not(target_arch = "wasm32"))]
            Command::SaveConfig => self.save_config(),
            #[cfg(not(target_arch = "wasm32"))]
            Command::OpenRecent(index) => {
                let path = self.state.ui.settings.recents[index].clone();
                platform::open_config(self.sender.clone(), path.into());
            }
            Command::Undo => self.undo(),
            Command::Redo => self.redo(),
            Command::SelectAll => self.select_all(),
            Command::RemoveSelected => self.remove_selected(),
            Command::RemoveSprite(index) => self.remove_sprite(index),
            Command::ClearAll => self.clear_all(),
            Command::FitPreview => self.state.ui.preview.fit(),
            Command::ActualSize => self.state.ui.preview.actual_size(),
            Command::SetZoom(percent) => self.state.ui.preview.set_zoom(percent),
            Command::ToggleSprites => {
                self.state.ui.settings.show_sprites = !self.state.ui.settings.show_sprites;
            }
            Command::ToggleInspector => self.state.ui.toggle_inspector(),
            Command::ShowAbout => self.state.ui.dialog = Some(Dialog::About),
            #[cfg(target_os = "macos")]
            Command::SetInterfaceScale(percent) => {
                self.state.ui.settings.ui_scale = f32::from(percent) / 100.0;
            }
            #[cfg(target_os = "macos")]
            Command::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Command::ToggleOutlines => {
                self.state.ui.settings.show_outlines = !self.state.ui.settings.show_outlines;
            }
            Command::ToggleJson => {
                self.state.ui.settings.display_json = !self.state.ui.settings.display_json;
            }
            Command::CopyJson => {
                if let Some(built) = &self.state.build.output {
                    let json = serde_json::to_string_pretty(&built.atlas_asset_json)
                        .unwrap_or_else(|_| built.atlas_asset_json.to_string());
                    ctx.copy_text(json);
                    self.state
                        .ui
                        .toasts
                        .push(ctx, Level::Info, "JSON copied to the clipboard");
                }
            }
        }
    }
}

impl eframe::App for Application {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // A capture opens throwaway files, which should not end up in the recent list.
        #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
        if self.debug_capture.is_some() {
            return;
        }
        self.state.ui.settings.save(storage);
    }

    #[cfg(target_os = "macos")]
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        raw_input
            .events
            .extend(platform::macos_menu::drain_text_events());
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        profile_scope!("logic");
        self.state
            .undoer
            .feed_state(ctx.input(|input| input.time), &self.state.project);
        self.sync_scale(ctx);
        self.sync_narrow(ctx);
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped.is_empty() {
            platform::handle_dropped(self.sender.clone(), dropped);
        }
        while let Some(event) = self.inbox.next() {
            self.handle_event(ctx, event);
        }
        keys::handle(ctx, self.state.ui.native_menu);
        self.run_commands(ctx);
        self.sync_build(ctx);
        self.sync_window_title(ctx);
        #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
        if let Some(capture) = &mut self.debug_capture {
            let build = &self.state.build;
            let ready = build.pending_at.is_none() && !build.building && build.last_key.is_some();
            capture.tick(ctx, ready && !self.state.project.image_data.is_empty());
        }
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ui::show(ui, &mut self.state);
        self.run_commands(&ctx);
        #[cfg(target_os = "macos")]
        if let Some(menu) = &mut self.native_menu {
            menu.sync(&self.state);
        }
    }
}
