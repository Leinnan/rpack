use egui::{Align2, Color32, Context, Frame, Id, Order, RichText, Sense, vec2};
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
    created: f64,
}

/// Short-lived messages drawn in the bottom right corner. Click a message to dismiss it.
#[derive(Default)]
pub struct Notifications {
    items: Vec<Toast>,
}

const MAX_VISIBLE: usize = 5;

impl Notifications {
    pub fn push(&mut self, ctx: &Context, level: Level, text: impl Into<String>) {
        let text = text.into();
        let created = ctx.input(|i| i.time);
        // Identical consecutive messages only refresh the timer instead of stacking up.
        if let Some(existing) = self
            .items
            .iter_mut()
            .find(|t| t.level == level && t.text == text)
        {
            existing.created = created;
            return;
        }
        self.items.push(Toast {
            level,
            text,
            created,
        });
        if self.items.len() > MAX_VISIBLE {
            self.items.remove(0);
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        let now = ctx.input(|i| i.time);
        self.items.retain(|t| now - t.created < t.level.lifetime());
        if self.items.is_empty() {
            return;
        }
        let mut dismissed = None;
        egui::Area::new(Id::unique("notifications"))
            .anchor(Align2::RIGHT_BOTTOM, vec2(-12.0, -48.0))
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
