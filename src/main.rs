//! LaReview - Intent-based Pull Request Review Tool
//!
//! A native desktop application built with GPUI for reviewing
//! pull requests by intent, not by file order.

mod acp;
mod data;
mod domain;
mod ui;

use gpui::{App, AppContext, Application, WindowOptions};
use ui::app::LaReviewApp;

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_, cx| {
            cx.new(|cx| LaReviewApp::new(cx))
        })
        .unwrap();
    });
}
