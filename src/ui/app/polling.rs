use super::LaReviewApp;
use crate::ui::app::{Action, AsyncAction};

impl LaReviewApp {
    pub(super) fn poll_d2_install_messages(&mut self) {
        while let Ok(msg) = self.d2_install_rx.try_recv() {
            if msg == "___INSTALL_COMPLETE___" {
                self.state.is_d2_installing = false;
            } else {
                self.state.d2_install_output.push_str(&msg);
                self.state.d2_install_output.push('\n');
            }
        }
    }

    pub(super) fn poll_generation_messages(&mut self) -> bool {
        let mut agent_content_updated = false;
        while let Ok(msg) = self.gen_rx.try_recv() {
            self.dispatch(Action::Async(AsyncAction::GenerationMessage(msg)));
            agent_content_updated = true;
        }
        agent_content_updated
    }
}
