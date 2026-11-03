use super::start_screen;
use super::theme::colors::{
    ACCENT, CANVAS_SHADOW, CHECKER_ALT, CHECKER_BASE, DELETE_RED_SOFT, PANEL_BG,
};
use crate::state::AppState;
use egui::{
    Frame, Margin, Pos2, Rect, RichText, Sense, Shadow, Stroke, StrokeKind, TextureOptions, Ui,
    UiBuilder, Vec2, vec2,
};
use rpack_core::Spritesheet;
use std::collections::HashSet;

pub const PREVIEW_URI: &str = "bytes://output.png";
const MIN_ZOOM: f32 = 0.05;
const MAX_ZOOM: f32 = 64.0;
const MAX_FIT_ZOOM: f32 = 4.0;
const CHECKER_CELL: f32 = 12.0;

/// Zoom and pan state of the atlas preview.
pub struct PreviewState {
    /// While set, the atlas is scaled to fill the viewport and centered.
    pub fit: bool,
    pub zoom: f32,
    pub offset: Vec2,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self {
            fit: true,
            zoom: 1.0,
            offset: Vec2::ZERO,
        }
    }
}

impl PreviewState {
    pub fn fit(&mut self) {
        self.fit = true;
        self.offset = Vec2::ZERO;
    }

    pub fn actual_size(&mut self) {
        self.fit = false;
        self.zoom = 1.0;
        self.offset = Vec2::ZERO;
    }

    pub fn set_zoom(&mut self, percent: u16) {
        self.fit = false;
        self.zoom = (f32::from(percent) / 100.0).clamp(MIN_ZOOM, MAX_ZOOM);
    }

    pub fn zoom_percent(&self) -> u32 {
        (self.zoom * 100.0).round() as u32
    }
}

fn fit_zoom(viewport: Vec2, atlas: Vec2) -> f32 {
    let available = (viewport - Vec2::splat(16.0)).max(Vec2::splat(1.0));
    (available.x / atlas.x)
        .min(available.y / atlas.y)
        .clamp(MIN_ZOOM, MAX_FIT_ZOOM)
}

fn paint_checkerboard(ui: &Ui, visible: Rect, origin: Pos2) {
    let (base, alt) = (CHECKER_BASE, CHECKER_ALT);
    let painter = ui.painter();
    painter.rect_filled(visible, 0.0, base);
    let first_x = ((visible.min.x - origin.x) / CHECKER_CELL).floor() as i32;
    let last_x = ((visible.max.x - origin.x) / CHECKER_CELL).ceil() as i32;
    let first_y = ((visible.min.y - origin.y) / CHECKER_CELL).floor() as i32;
    let last_y = ((visible.max.y - origin.y) / CHECKER_CELL).ceil() as i32;
    for y in first_y..last_y {
        for x in first_x..last_x {
            if (x + y).rem_euclid(2) == 1 {
                let cell = Rect::from_min_size(
                    origin + vec2(x as f32, y as f32) * CHECKER_CELL,
                    Vec2::splat(CHECKER_CELL),
                )
                .intersect(visible);
                if cell.is_positive() {
                    painter.rect_filled(cell, 0.0, alt);
                }
            }
        }
    }
}

/// The atlas, or the start screen and any error instead of it.
pub fn show_panel(ui: &mut Ui, state: &mut AppState) {
    profile_scope!("preview_panel");
    egui::CentralPanel::default()
        .frame(Frame::new().fill(PANEL_BG))
        .show(ui, |ui| {
            if let Some(error) = &state.build.last_error {
                Frame::new().inner_margin(Margin::same(10)).show(ui, |ui| {
                    ui.label(RichText::new(error.to_string()).color(DELETE_RED_SOFT));
                });
            }
            if state.project.image_data.is_empty() {
                start_screen::show(ui, state);
                return;
            }
            if let Some(built) = &state.build.output {
                let selected_keys: HashSet<&str> = state
                    .project
                    .image_data
                    .iter()
                    .filter(|image| state.ui.selected.contains(&image.path))
                    .map(|image| image.id())
                    .collect();
                let hovered = show(
                    ui,
                    &mut state.ui.preview,
                    built,
                    state.ui.settings.show_outlines,
                    state.build.building,
                    &selected_keys,
                );
                if hovered != state.ui.hovered_frame {
                    state.ui.hovered_frame = hovered;
                    ui.ctx().request_repaint();
                }
            } else if state.build.building {
                ui.centered_and_justified(|ui| ui.spinner());
            }
        });
}

/// Draws the atlas with scroll-to-zoom, drag-to-pan and sprite outlines.
/// `building` dims the preview while a new atlas is being packed.
fn show(
    ui: &mut Ui,
    state: &mut PreviewState,
    sheet: &Spritesheet,
    show_outlines: bool,
    building: bool,
    selected_keys: &HashSet<&str>,
) -> Option<String> {
    let atlas_size = vec2(
        sheet.atlas_asset.size[0].max(1) as f32,
        sheet.atlas_asset.size[1].max(1) as f32,
    );
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    if state.fit {
        state.zoom = fit_zoom(rect.size(), atlas_size);
        state.offset = Vec2::ZERO;
    }

    if response.hovered() {
        let (scroll, pinch, pointer) = ui.input(|i| {
            (
                i.smooth_scroll_delta.y,
                i.zoom_delta(),
                i.pointer.hover_pos(),
            )
        });
        let factor = pinch * (scroll * 0.0025).exp();
        if (factor - 1.0).abs() > f32::EPSILON {
            let new_zoom = (state.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
            let ratio = new_zoom / state.zoom;
            // Keep the point under the cursor fixed while zooming.
            let anchor = pointer.unwrap_or_else(|| rect.center());
            let center = rect.center() + state.offset;
            state.offset = anchor + (center - anchor) * ratio - rect.center();
            state.zoom = new_zoom;
            state.fit = false;
        }
    }
    if response.dragged() {
        state.offset += response.drag_delta();
        state.fit = false;
    }
    if response.double_clicked() {
        state.fit();
    }

    let image_rect = Rect::from_center_size(rect.center() + state.offset, atlas_size * state.zoom);
    let mut canvas = ui.new_child(UiBuilder::new().max_rect(rect));
    canvas.shrink_clip_rect(rect);
    canvas.painter().add(
        Shadow {
            offset: [0, 2],
            blur: 12,
            spread: 0,
            color: CANVAS_SHADOW,
        }
        .as_shape(image_rect, 0.0),
    );
    let visible = image_rect.intersect(rect);
    if visible.is_positive() {
        paint_checkerboard(&canvas, visible, image_rect.min);
    }
    let options = if state.zoom >= 1.0 {
        TextureOptions::NEAREST
    } else {
        TextureOptions::LINEAR
    };
    egui::Image::new(PREVIEW_URI)
        .texture_options(options)
        .paint_at(&canvas, image_rect);

    let to_screen = |x: u32, y: u32, w: u32, h: u32| {
        Rect::from_min_size(
            image_rect.min + vec2(x as f32, y as f32) * state.zoom,
            vec2(w as f32, h as f32) * state.zoom,
        )
    };
    if show_outlines {
        let stroke = Stroke::new(1.0, ACCENT.gamma_multiply(0.6));
        for frame in &sheet.atlas_asset.frames {
            let r = &frame.frame;
            canvas.painter().rect_stroke(
                to_screen(r.x, r.y, r.w, r.h),
                0.0,
                stroke,
                StrokeKind::Inside,
            );
        }
    }
    if !selected_keys.is_empty() {
        for frame in &sheet.atlas_asset.frames {
            if selected_keys.contains(frame.key.as_str()) {
                let r = &frame.frame;
                let frame_rect = to_screen(r.x, r.y, r.w, r.h);
                canvas
                    .painter()
                    .rect_filled(frame_rect, 0.0, ACCENT.gamma_multiply(0.25));
                canvas.painter().rect_stroke(
                    frame_rect,
                    0.0,
                    Stroke::new(2.0, ACCENT),
                    StrokeKind::Inside,
                );
            }
        }
    }
    let mut hovered_key = None;
    if !building && let Some(pointer) = response.hover_pos() {
        let atlas_pos = (pointer - image_rect.min) / state.zoom;
        let hovered = sheet.atlas_asset.frames.iter().find(|frame| {
            let r = &frame.frame;
            atlas_pos.x >= r.x as f32
                && atlas_pos.y >= r.y as f32
                && atlas_pos.x < (r.x + r.w) as f32
                && atlas_pos.y < (r.y + r.h) as f32
        });
        if let Some(frame) = hovered {
            let r = &frame.frame;
            canvas.painter().rect_stroke(
                to_screen(r.x, r.y, r.w, r.h),
                0.0,
                Stroke::new(2.0, ACCENT),
                StrokeKind::Inside,
            );
            response.clone().on_hover_text(format!(
                "{}\n{}x{} at ({}, {})",
                frame.key, r.w, r.h, r.x, r.y
            ));
            hovered_key = Some(frame.key.clone());
        }
    }

    if building {
        canvas.painter().rect_filled(rect, 0.0, CANVAS_SHADOW);
        let spinner_rect = Rect::from_center_size(rect.center(), Vec2::splat(32.0));
        ui.put(spinner_rect, egui::Spinner::new().size(32.0));
        ui.ctx().request_repaint();
    }
    hovered_key
}
