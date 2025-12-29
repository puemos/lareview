//! Review view module (split by feature parts).

mod center_pane;
mod feedback;
mod feedback_detail;
pub mod feedback_list;
mod nav;
mod task;
mod task_detail;
mod toolbar;
mod view;
mod visuals;

pub(super) fn format_timestamp(value: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| {
            dt.with_timezone(&chrono::Local)
                .format("%b %d, %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| value.to_string())
}
