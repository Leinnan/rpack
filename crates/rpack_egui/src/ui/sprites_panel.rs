//! The list of source images.
use super::theme::colors::{CARD_BG_HOVER, HINT_TEXT, SELECTION_BG, TEXT, TEXT_WEAK};
use super::widgets::{
    buttons::{Tip, small_icon_button},
    paint_line,
    section::section_header,
};
use crate::commands::Command;
use crate::state::{AppImageData, AppState};
use egui::{
    Align, Align2, CornerRadius, Image, Layout, Rect, Response, RichText, Role, Sense, Ui,
    UiBuilder, Vec2, WidgetInfo, vec2,
};
use lucide_icons::Icon;

const ROW_HEIGHT: f32 = 40.0;
const ROW_GAP: f32 = 2.0;
const THUMBNAIL: f32 = 32.0;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    profile_scope!("sprites_panel");
    let count = state.project.image_data.len();
    ui.horizontal(|ui| {
        section_header(ui, &format!("Sprites  {count}"));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let clear = Command::ClearAll;
            if small_icon_button(ui, Icon::ListX, clear.enabled(state), "Remove all images")
                .clicked()
            {
                clear.schedule();
            }
            let add = Tip::command("Add more images", Command::AddImages);
            if small_icon_button(ui, Icon::Plus, true, add).clicked() {
                Command::AddImages.schedule();
            }
        });
    });
    ui.add_space(2.0);
    if count == 0 {
        ui.label(RichText::new("No images yet").color(HINT_TEXT));
        return;
    }
    list(ui, state);
}

fn list(ui: &mut Ui, state: &mut AppState) {
    let images = &state.project.image_data;
    let pitch = ROW_HEIGHT + ROW_GAP;
    let mut area = egui::ScrollArea::vertical()
        .id_salt("sprites_list")
        .auto_shrink(false);
    // Bring the sprite hovered in the preview into view, once per hover.
    let hovered = state.ui.hovered_frame.as_deref();
    let scroll_to = hovered
        .filter(|id| state.ui.scrolled_to.as_deref() != Some(*id))
        .and_then(|id| images.iter().position(|image| image.id() == id));
    if let Some(index) = scroll_to {
        let top = index as f32 * pitch;
        let (offset, view) = (state.ui.sprites_offset, state.ui.sprites_view);
        if top < offset || top + ROW_HEIGHT > offset + view {
            area = area.vertical_scroll_offset((top - (view - ROW_HEIGHT) / 2.0).max(0.0));
        }
    }
    let mut clicked = None;
    let output = area.show_rows(ui, ROW_HEIGHT, images.len(), |ui, rows| {
        ui.spacing_mut().item_spacing.y = ROW_GAP;
        for index in rows {
            let image = &images[index];
            let selected = state.ui.selected.contains(&image.path);
            let highlighted = hovered == Some(image.id());
            let response = row(ui, index, image, selected, highlighted);
            if response.clicked() {
                clicked = Some(index);
            }
            response.context_menu(|ui| context_menu(ui, index, selected));
        }
    });
    state.ui.sprites_offset = output.state.clamped_offset().y;
    state.ui.sprites_view = output.inner_rect.height();
    state.ui.scrolled_to = state.ui.hovered_frame.clone();
    if let Some(index) = clicked {
        let modifiers = ui.input(|i| i.modifiers);
        state
            .ui
            .click_sprite(index, modifiers, &state.project.image_data);
    }
}

fn row(
    ui: &mut Ui,
    index: usize,
    image: &AppImageData,
    selected: bool,
    highlighted: bool,
) -> Response {
    let size = vec2(ui.available_width(), ROW_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let response =
        response.on_hover_text(format!("{}×{}\n{}", image.width, image.height, image.path));
    response.widget_info(|| WidgetInfo::selected(Role::ListBoxOption, true, selected, image.id()));
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let hovered = ui.rect_contains_pointer(rect);
    let fill = if selected {
        Some(SELECTION_BG)
    } else if hovered || highlighted {
        Some(CARD_BG_HOVER)
    } else {
        None
    };
    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
    }

    let scale = THUMBNAIL / image.width.max(image.height).max(1) as f32;
    let thumbnail = Rect::from_center_size(
        rect.left_center() + vec2(4.0 + THUMBNAIL / 2.0, 0.0),
        vec2(image.width as f32, image.height as f32) * scale.min(1.0),
    );
    Image::from_uri(image.thumbnail_uri()).paint_at(ui, thumbnail);

    let show_remove = hovered || selected;
    let right = if show_remove { 30.0 } else { 6.0 };
    let label_rect = Rect::from_min_max(
        rect.left_top() + vec2(THUMBNAIL + 12.0, 0.0),
        rect.right_bottom() - vec2(right, 0.0),
    );
    let color = if selected { TEXT } else { TEXT_WEAK };
    paint_line(
        ui,
        label_rect,
        Align2::LEFT_CENTER,
        RichText::new(image.id()),
        color,
    );

    if show_remove {
        let button_rect =
            Rect::from_center_size(rect.right_center() - vec2(15.0, 0.0), Vec2::splat(22.0));
        let mut button = ui.new_child(
            UiBuilder::new()
                .max_rect(button_rect)
                .id_salt(("remove", index)),
        );
        if small_icon_button(&mut button, Icon::X, true, "Remove").clicked() {
            Command::RemoveSprite(index).schedule();
        }
    }
    response
}

fn context_menu(ui: &mut Ui, index: usize, selected: bool) {
    if ui.button("Remove").clicked() {
        if selected {
            Command::RemoveSelected.schedule();
        } else {
            Command::RemoveSprite(index).schedule();
        }
        ui.close();
    }
    if ui.button("Select all").clicked() {
        Command::SelectAll.schedule();
        ui.close();
    }
}
