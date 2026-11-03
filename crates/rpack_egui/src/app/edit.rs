//! Undo and changes to the image list.
use super::Application;

impl Application {
    pub(super) fn undo(&mut self) {
        if let Some(state) = self.state.undoer.undo(&self.state.project) {
            self.state.project = state.clone();
        }
    }

    pub(super) fn redo(&mut self) {
        if let Some(state) = self.state.undoer.redo(&self.state.project) {
            self.state.project = state.clone();
        }
    }

    pub(super) fn select_all(&mut self) {
        self.state.ui.selected = self
            .state
            .project
            .image_data
            .iter()
            .map(|image| image.path.clone())
            .collect();
    }

    pub(super) fn remove_selected(&mut self) {
        let selected = &self.state.ui.selected;
        self.state
            .project
            .image_data
            .retain(|image| !selected.contains(&image.path));
        self.state.project.update_ids();
    }

    pub(super) fn remove_sprite(&mut self, index: usize) {
        self.state.project.image_data.remove(index);
        self.state.project.update_ids();
    }

    pub(super) fn clear_all(&mut self) {
        self.state.project.image_data.clear();
    }
}
