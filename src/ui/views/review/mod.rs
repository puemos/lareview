//! Review view module (split by feature parts).

mod nav;
mod screen;
mod selection;
mod task_detail;
mod thread_detail;

pub(super) fn format_timestamp(value: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| {
            dt.with_timezone(&chrono::Local)
                .format("%b %d, %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| value.to_string())
}
