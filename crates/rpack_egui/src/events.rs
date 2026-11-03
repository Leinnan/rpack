//! Results sent from worker threads and async file dialogs back to the UI.
use crate::state::{AppImageData, BuildOutcome};
use crate::ui::toasts::Level;
#[cfg(not(target_arch = "wasm32"))]
use rpack_core::TilemapGenerationConfig;
use std::sync::mpsc;

pub enum AppEvent {
    Images(Vec<AppImageData>),
    /// A build finished. Results of builds that were superseded are dropped.
    Build(u64, BuildOutcome),
    Notify(Level, String),
    /// A config and its images were read. They replace the current project.
    #[cfg(not(target_arch = "wasm32"))]
    Config {
        config: TilemapGenerationConfig,
        path: String,
        images: Vec<AppImageData>,
    },
    /// The config file could not be read, so it should not stay in the recent list.
    #[cfg(not(target_arch = "wasm32"))]
    ConfigUnreadable {
        path: String,
        error: String,
    },
    #[cfg(not(target_arch = "wasm32"))]
    AddRecent(String),
}

/// Sends events to the UI and wakes it up, so they are handled without waiting for input.
#[derive(Clone)]
pub struct EventSender {
    tx: mpsc::Sender<AppEvent>,
    ctx: egui::Context,
}

impl EventSender {
    pub fn send(&self, event: AppEvent) {
        // The receiver only goes away while the app shuts down.
        let _ = self.tx.send(event);
        self.ctx.request_repaint();
    }

    pub fn notify(&self, level: Level, text: impl Into<String>) {
        self.send(AppEvent::Notify(level, text.into()));
    }
}

pub struct EventInbox {
    rx: mpsc::Receiver<AppEvent>,
}

impl EventInbox {
    pub fn next(&self) -> Option<AppEvent> {
        self.rx.try_recv().ok()
    }
}

pub fn channel(ctx: &egui::Context) -> (EventSender, EventInbox) {
    let (tx, rx) = mpsc::channel();
    let sender = EventSender {
        tx,
        ctx: ctx.clone(),
    };
    (sender, EventInbox { rx })
}
