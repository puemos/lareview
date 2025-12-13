//! Reducer-style state updates + side-effect commands.

mod action;
mod command;
mod reducer;
mod runtime;

pub use action::{
    Action, AsyncAction, GenerateAction, NavigationAction, ReviewAction, SettingsAction,
};
pub use command::ReviewDataRefreshReason;

use super::LaReviewApp;

impl LaReviewApp {
    pub fn dispatch(&mut self, action: Action) {
        let commands = reducer::reduce(&mut self.state, action);
        for command in commands {
            runtime::run(self, command);
        }
    }
}
