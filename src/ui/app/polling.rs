use super::LaReviewApp;
use crate::ui::app::{Action, AsyncAction};

impl LaReviewApp {
    pub(super) fn poll_gh_messages(&mut self) {
        while let Ok(msg) = self.gh_rx.try_recv() {
            match msg {
                crate::ui::app::GhMsg::Done(result) => {
                    self.dispatch(Action::Async(AsyncAction::GhStatusLoaded(result)));
                }
            }
        }
    }

    pub(super) fn poll_d2_install_messages(&mut self) {
        while let Ok(msg) = self.d2_install_rx.try_recv() {
            if msg == "___INSTALL_COMPLETE___" {
                self.dispatch(Action::Async(AsyncAction::D2InstallComplete));
            } else {
                self.dispatch(Action::Async(AsyncAction::D2InstallOutput(msg)));
            }
        }
    }

    pub(super) fn poll_generation_messages(&mut self) -> bool {
        let mut agent_content_updated = false;
        while let Ok(msg) = self.gen_rx.try_recv() {
            self.dispatch(Action::Async(AsyncAction::GenerationMessage(Box::new(msg))));
            agent_content_updated = true;
        }
        agent_content_updated
    }

    pub(super) fn poll_action_messages(&mut self) -> bool {
        let mut any = false;
        while let Ok(action) = self.action_rx.try_recv() {
            self.dispatch(action);
            any = true;
        }
        any
    }
}
