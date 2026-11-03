use egui::{Align2, Color32, Context, Frame, Id, Order, RichText, Sense, pos2};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Info,
    Warning,
    Error,
}

impl Level {
    fn lifetime(self) -> f64 {
        match self {
            Level::Info => 3.5,
            Level::Warning => 6.0,
            Level::Error => 9.0,
        }
    }

    fn color(self, visuals: &egui::Visuals) -> Color32 {
        match self {
            Level::Info => visuals.text_color(),
            Level::Warning => visuals.warn_fg_color,
            Level::Error => visuals.error_fg_color,
        }
    }

    fn prefix(self) -> &'static str {
        match self {
            Level::Info => "",
            Level::Warning => "Warning: ",
            Level::Error => "Error: ",
        }
    }
}

struct Toast {
    level: Level,
    text: String,
    /// Set by the first frame showing it. Pushes can happen before that frame's time
    /// is known, and no frames run while a native dialog is open.
    shown_at: Option<f64>,
}

/// Short-lived messages drawn in the bottom right corner. Click a message to dismiss it.
#[derive(Default)]
pub struct Notifications {
    items: Vec<Toast>,
}

const MAX_VISIBLE: usize = 5;

impl Notifications {
    pub fn push(&mut self, level: Level, text: impl Into<String>) {
        let text = text.into();
        // Identical consecutive messages only refresh the timer instead of stacking up.
        if let Some(existing) = self
            .items
            .iter_mut()
            .find(|t| t.level == level && t.text == text)
        {
            existing.shown_at = None;
            return;
        }
        self.items.push(Toast {
            level,
            text,
            shown_at: None,
        });
        if self.items.len() > MAX_VISIBLE {
            self.items.remove(0);
        }
    }

    /// Draws the messages above `bottom`, the top of the status bar.
    pub fn show(&mut self, ctx: &Context, bottom: f32) {
        let now = ctx.input(|i| i.time);
        self.items
            .retain_mut(|t| now - *t.shown_at.get_or_insert(now) < t.level.lifetime());
        if self.items.is_empty() {
            return;
        }
        let mut dismissed = None;
        egui::Area::new(Id::unique("notifications"))
            .pivot(Align2::RIGHT_BOTTOM)
            .fixed_pos(pos2(ctx.content_rect().right() - 12.0, bottom - 12.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                ui.set_max_width(360.0);
                for (index, toast) in self.items.iter().enumerate() {
                    let response = Frame::popup(ui.style()).show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("{}{}", toast.level.prefix(), toast.text))
                                    .color(toast.level.color(ui.visuals())),
                            )
                            .selectable(false),
                        );
                    });
                    let response = response
                        .response
                        .interact(Sense::click())
                        .on_hover_text("Click to dismiss");
                    if response.clicked() {
                        dismissed = Some(index);
                    }
                    ui.add_space(4.0);
                }
            });
        if let Some(index) = dismissed {
            self.items.remove(index);
        }
        ctx.request_repaint_after(Duration::from_millis(250));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show_at(ctx: &Context, toasts: &mut Notifications, time: f64) {
        let input = egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| toasts.show(ui.ctx(), 100.0));
        output.textures_delta.clear();
    }

    #[test]
    fn lifetime_starts_at_the_first_frame_showing_the_message() {
        let ctx = Context::default();
        let mut toasts = Notifications::default();
        show_at(&ctx, &mut toasts, 1.0);
        toasts.push(Level::Info, "Saved atlas.png");
        // No frames ran for a while, as with a native save dialog open.
        show_at(&ctx, &mut toasts, 60.0);
        assert_eq!(toasts.items.len(), 1);
        show_at(&ctx, &mut toasts, 63.0);
        assert_eq!(toasts.items.len(), 1);
        show_at(&ctx, &mut toasts, 64.0);
        assert!(toasts.items.is_empty());
    }

    #[test]
    fn repeating_a_message_restarts_its_lifetime() {
        let ctx = Context::default();
        let mut toasts = Notifications::default();
        toasts.push(Level::Info, "Saved");
        show_at(&ctx, &mut toasts, 1.0);
        show_at(&ctx, &mut toasts, 4.0);
        toasts.push(Level::Info, "Saved");
        show_at(&ctx, &mut toasts, 6.0);
        assert_eq!(toasts.items.len(), 1);
    }
}
