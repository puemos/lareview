use crate::ui::app::{Action, LaReviewApp, ReviewAction};

impl LaReviewApp {
    pub(super) fn select_task(&mut self, task: &crate::domain::ReviewTask) {
        self.dispatch(Action::Review(ReviewAction::SelectTask {
            task_id: task.id.clone(),
        }));
    }
}
