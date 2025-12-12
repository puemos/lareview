use crate::ui::app::{Action, GenerateAction, LaReviewApp};

impl LaReviewApp {
    pub(super) fn reset_generation_state(&mut self) {
        self.dispatch(Action::Generate(GenerateAction::Reset));
    }

    pub fn start_generation_async(&mut self) {
        self.dispatch(Action::Generate(GenerateAction::RunRequested));
    }
}
