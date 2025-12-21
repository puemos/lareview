use super::super::super::state::{AppState, AppView};
use super::super::action::NavigationAction;
use super::super::command::{Command, ReviewDataRefreshReason};

pub fn reduce(state: &mut AppState, action: NavigationAction) -> Vec<Command> {
    match action {
        NavigationAction::SwitchTo(view) => {
            state.active_thread = None;
            state.current_view = view;
            if matches!(view, AppView::Review) {
                return vec![Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::Navigation,
                }];
            }
            if matches!(view, AppView::Settings) {
                // If we haven't checked GitHub status yet, trigger it
                if state.gh_status.is_none()
                    && state.gh_status_error.is_none()
                    && !state.is_gh_status_checking
                {
                    state.is_gh_status_checking = true;
                    return vec![Command::CheckGitHubStatus];
                }
            }
            Vec::new()
        }
    }
}
