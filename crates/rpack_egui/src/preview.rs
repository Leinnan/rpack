use egui::{
    Color32, Pos2, Rect, Sense, Stroke, StrokeKind, TextureOptions, Ui, UiBuilder, Vec2, vec2,
};
use rpack_cli::Spritesheet;
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
    let (base, alt) = if ui.visuals().dark_mode {
        (Color32::from_gray(48), Color32::from_gray(64))
    } else {
        (Color32::from_gray(205), Color32::from_gray(232))
    };
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

/// Draws the atlas with scroll-to-zoom, drag-to-pan and sprite outlines.
/// `building` dims the preview while a new atlas is being packed.
pub fn show(
    ui: &mut Ui,
    state: &mut PreviewState,
    sheet: &Spritesheet,
    show_outlines: bool,
    building: bool,
    accent: Color32,
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
    canvas
        .painter()
        .rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
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
        let stroke = Stroke::new(1.0, accent.gamma_multiply(0.6));
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
    if show_outlines && !selected_keys.is_empty() {
        for frame in &sheet.atlas_asset.frames {
            if selected_keys.contains(frame.key.as_str()) {
                let r = &frame.frame;
                let frame_rect = to_screen(r.x, r.y, r.w, r.h);
                canvas
                    .painter()
                    .rect_filled(frame_rect, 0.0, accent.gamma_multiply(0.25));
                canvas.painter().rect_stroke(
                    frame_rect,
                    0.0,
                    Stroke::new(2.0, accent),
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
                Stroke::new(2.0, accent),
                StrokeKind::Inside,
            );
            response.clone().on_hover_text(format!(
                "{}\n{}x{} at ({}, {})",
                frame.key, r.w, r.h, r.x, r.y
            ));
            if show_outlines {
                hovered_key = Some(frame.key.clone());
            }
        }
    }

    if building {
        canvas
            .painter()
            .rect_filled(rect, 0.0, ui.visuals().extreme_bg_color.gamma_multiply(0.6));
        let spinner_rect = Rect::from_center_size(rect.center(), Vec2::splat(32.0));
        ui.put(spinner_rect, egui::Spinner::new().size(32.0));
        ui.ctx().request_repaint();
    }
    hovered_key
}
