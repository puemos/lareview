use super::super::super::state::{AppView, SessionState, UiState};
use super::super::action::NavigationAction;
use super::super::command::{Command, ReviewDataRefreshReason};

pub fn reduce(
    ui: &mut UiState,
    session: &mut SessionState,
    action: NavigationAction,
) -> Vec<Command> {
    match action {
        NavigationAction::SwitchTo(view) => {
            ui.active_thread = None;
            ui.current_view = view;
            if matches!(view, AppView::Review) {
                return vec![Command::RefreshReviewData {
                    reason: ReviewDataRefreshReason::Navigation,
                }];
            }
            if matches!(view, AppView::Settings) {
                // If we haven't checked GitHub status yet, trigger it
                if session.gh_status.is_none()
                    && session.gh_status_error.is_none()
                    && !session.is_gh_status_checking
                {
                    session.is_gh_status_checking = true;
                    return vec![Command::CheckGitHubStatus];
                }
            }
            Vec::new()
        }
    }
}
