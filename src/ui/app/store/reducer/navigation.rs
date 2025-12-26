use super::super::super::state::{AppView, SessionState, UiState};
use super::super::action::NavigationAction;
use super::super::command::{Command, ReviewDataRefreshReason};

pub fn reduce(
    ui: &mut UiState,
    _session: &mut SessionState,
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
            Vec::new()
        }
    }
}
