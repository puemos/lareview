use crate::ui::app::{Action, LaReviewApp, ReviewAction};

impl LaReviewApp {
    pub(super) fn select_task(&mut self, task: &crate::domain::ReviewTask) {
        self.dispatch(Action::Review(ReviewAction::SelectTask {
            task_id: task.id.clone(),
        }));
    }

    pub(super) fn select_task_by_id(
        &mut self,
        all_tasks: &[crate::domain::ReviewTask],
        task_id: &str,
    ) {
        if all_tasks.iter().any(|t| t.id == task_id) {
            self.dispatch(Action::Review(ReviewAction::SelectTaskById {
                task_id: task_id.to_string(),
            }));
        }
    }
}
