use crate::domain::{ReviewStatus, TaskId};

use super::state::AppView;
use super::{Action, LaReviewApp, NavigationAction, ReviewAction};
use crate::ui::app::store::ReviewDataRefreshReason;

impl LaReviewApp {
    pub fn switch_to_review(&mut self) {
        self.dispatch(Action::Navigation(NavigationAction::SwitchTo(
            AppView::Review,
        )));
    }

    pub fn switch_to_generate(&mut self) {
        self.dispatch(Action::Navigation(NavigationAction::SwitchTo(
            AppView::Generate,
        )));
    }

    pub fn switch_to_repos(&mut self) {
        self.dispatch(Action::Navigation(NavigationAction::SwitchTo(
            AppView::Repos,
        )));
    }

    pub fn switch_to_settings(&mut self) {
        self.dispatch(Action::Navigation(NavigationAction::SwitchTo(
            AppView::Settings,
        )));
    }

    pub fn sync_review_from_db(&mut self) {
        self.dispatch(Action::Review(ReviewAction::RefreshFromDb {
            reason: ReviewDataRefreshReason::Manual,
        }));
    }

    pub fn set_task_status(&mut self, task_id: &TaskId, new_status: ReviewStatus) {
        self.dispatch(Action::Review(ReviewAction::UpdateTaskStatus {
            task_id: task_id.clone(),
            status: new_status,
        }));
    }
}
