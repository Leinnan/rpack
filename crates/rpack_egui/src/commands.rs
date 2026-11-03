//! User intents. Panels and shortcuts schedule them, `Application` applies them.
use crate::state::AppState;
use std::cell::RefCell;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Command {
    AddImages,
    #[cfg(not(target_arch = "wasm32"))]
    OpenConfig,
    SaveAtlas,
    SaveJson,
    #[cfg(not(target_arch = "wasm32"))]
    SaveConfig,
    #[cfg(not(target_arch = "wasm32"))]
    OpenRecent(usize),
    Undo,
    Redo,
    SelectAll,
    RemoveSelected,
    RemoveSprite(usize),
    ClearAll,
    FitPreview,
    ActualSize,
    SetZoom(u16),
    ToggleOutlines,
    ToggleJson,
    ToggleSprites,
    ToggleInspector,
    CopyJson,
    ShowAbout,
    #[cfg(target_os = "macos")]
    SetInterfaceScale(u16),
    #[cfg(target_os = "macos")]
    Quit,
}

// One queue per thread keeps parallel UI tests from draining each other's commands.
// Background work reports through `EventSender` instead.
thread_local! {
    static QUEUE: RefCell<Vec<Command>> = const { RefCell::new(Vec::new()) };
}

impl Command {
    pub fn schedule(self) {
        QUEUE.with(|queue| queue.borrow_mut().push(self));
    }

    /// Whether the command can do anything right now. Greys out menus and buttons.
    pub fn enabled(self, state: &AppState) -> bool {
        let has_images = !state.project.image_data.is_empty();
        match self {
            Command::SaveAtlas | Command::SaveJson => state.can_save(),
            #[cfg(not(target_arch = "wasm32"))]
            Command::SaveConfig => has_images,
            #[cfg(not(target_arch = "wasm32"))]
            Command::OpenRecent(index) => index < state.ui.settings.recents.len(),
            Command::Undo => state.undoer.has_undo(&state.project),
            Command::Redo => state.undoer.has_redo(&state.project),
            Command::SelectAll | Command::ClearAll => has_images,
            Command::RemoveSelected => !state.ui.selected.is_empty(),
            Command::RemoveSprite(index) => index < state.project.image_data.len(),
            Command::FitPreview | Command::ActualSize | Command::SetZoom(_) | Command::CopyJson => {
                state.build.output.is_some()
            }
            #[cfg(not(target_arch = "wasm32"))]
            Command::OpenConfig => true,
            #[cfg(target_os = "macos")]
            Command::SetInterfaceScale(_) | Command::Quit => true,
            Command::AddImages
            | Command::ToggleOutlines
            | Command::ToggleJson
            | Command::ToggleSprites
            | Command::ToggleInspector
            | Command::ShowAbout => true,
        }
    }

    /// Text fields get every other shortcut, such as their own undo and select all.
    pub fn works_while_typing(self) -> bool {
        matches!(
            self,
            Command::AddImages | Command::SaveAtlas | Command::SaveJson
        )
    }
}

pub fn drain() -> Vec<Command> {
    QUEUE.with(|queue| std::mem::take(&mut *queue.borrow_mut()))
}
