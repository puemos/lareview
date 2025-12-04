//! LaReview - Intent-based Pull Request Review Tool
//!
//! A native desktop application built with GPUI for reviewing
//! pull requests by intent, not by file order.

mod acp;
mod data;
mod domain;
mod ui;

use gpui::prelude::*;
use gpui::{px, size, App, AppContext, Application, Bounds, VisualContext, WindowBounds, WindowOptions};
use ui::app::LaReviewApp;

fn main() {
    Application::new().run(|cx| {
        cx.open_window(WindowOptions::default(), |_, cx: &mut WindowContext| {
            cx.new_view(|cx| LaReviewApp::new(cx))
        })
        .unwrap();
    });
}
