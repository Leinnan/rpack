//! Debug-build helper for checking rendering on the real backend:
//! `RPACK_SCREENSHOT=out.png rpack_egui atlas.rpack_gen.json`
//! waits for the atlas, renders a few frames, saves the window to `out.png` and quits.
use egui::ViewportCommand;
use std::path::PathBuf;

/// Frames to let the UI settle before capturing.
const SETTLE_FRAMES: u32 = 5;

#[derive(Debug)]
pub struct DebugCapture {
    path: PathBuf,
    frame: u32,
    requested: bool,
}

impl DebugCapture {
    /// The capture requested via `RPACK_SCREENSHOT`, if any.
    pub fn from_env() -> Option<Self> {
        let path = std::env::var_os("RPACK_SCREENSHOT")?;
        Some(Self {
            path: path.into(),
            frame: 0,
            requested: false,
        })
    }

    /// Advance one frame: once `ready` has held for a few frames, save a screenshot
    /// and close the window.
    pub fn tick(&mut self, ctx: &egui::Context, ready: bool) {
        ctx.request_repaint();
        if ready {
            self.frame += 1;
        }
        if self.requested || self.frame < SETTLE_FRAMES {
            return;
        }
        self.requested = true;
        let path = self.path.clone();
        let repaint = ctx.clone();
        ctx.request_screenshot(move |shot| {
            let [width, height] = shot.size;
            let pixels = shot.pixels.iter().flat_map(|p| p.to_srgba_unmultiplied());
            let image = image::RgbaImage::from_raw(width as u32, height as u32, pixels.collect());
            match image.map(|image| image.save(&path)) {
                Some(Ok(())) => log::info!("Saved screenshot to {}", path.display()),
                Some(Err(error)) => log::error!("Screenshot failed: {error}"),
                None => log::error!("Screenshot failed: unexpected buffer size"),
            }
            repaint.send_viewport_cmd(ViewportCommand::Close);
            repaint.request_repaint();
        });
    }
}
