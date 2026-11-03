//! Window title and interface scale.
use super::Application;

const INSPECTOR_MIN_WINDOW: f32 = 1000.0;

impl Application {
    pub(super) fn sync_window_title(&mut self, ctx: &egui::Context) {
        let project = &self.state.project;
        let title = if project.image_data.is_empty() {
            "rPack".to_owned()
        } else {
            format!("rPack - {}", project.settings.output_path)
        };
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }

    /// The inspector hides on a narrow window and returns once it is wide again.
    pub(super) fn sync_narrow(&mut self, ctx: &egui::Context) {
        let narrow = ctx.content_rect().width() < INSPECTOR_MIN_WINDOW;
        if self.state.ui.narrow && !narrow {
            self.state.ui.inspector_forced = false;
        }
        self.state.ui.narrow = narrow;
    }

    /// Applies the interface scale, once the slider setting it is let go.
    pub(super) fn sync_scale(&mut self, ctx: &egui::Context) {
        let scale = self.state.ui.settings.ui_scale;
        let dragging = ctx.input(|i| i.pointer.any_down());
        if !dragging && (scale - self.applied_scale).abs() > f32::EPSILON {
            ctx.set_zoom_factor(scale);
            self.applied_scale = scale;
        }
    }
}
