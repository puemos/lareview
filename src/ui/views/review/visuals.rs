use crate::domain::{ThreadImpact, ThreadStatus};
use crate::ui::icons;
use crate::ui::theme::Theme;
use eframe::egui;

pub struct Visuals {
    pub label: &'static str,
    pub icon: &'static str,
    pub color: egui::Color32,
}

pub fn status_visuals(status: ThreadStatus, theme: &Theme) -> Visuals {
    match status {
        ThreadStatus::Todo => Visuals {
            label: "Todo",
            icon: icons::STATUS_TODO,
            color: theme.brand,
        },
        ThreadStatus::Wip => Visuals {
            label: "Wip",
            icon: icons::STATUS_WIP,
            color: theme.accent,
        },
        ThreadStatus::Done => Visuals {
            label: "Done",
            icon: icons::STATUS_DONE,
            color: theme.success,
        },
        ThreadStatus::Reject => Visuals {
            label: "Reject",
            icon: icons::STATUS_REJECTED,
            color: theme.destructive,
        },
    }
}

pub fn impact_visuals(impact: ThreadImpact, theme: &Theme) -> Visuals {
    match impact {
        ThreadImpact::Blocking => Visuals {
            label: "Blocking",
            icon: icons::IMPACT_BLOCKING,
            color: theme.destructive,
        },
        ThreadImpact::NiceToHave => Visuals {
            label: "Nice to have",
            icon: icons::IMPACT_NICE_TO_HAVE,
            color: theme.brand,
        },
        ThreadImpact::Nitpick => Visuals {
            label: "Nitpick",
            icon: icons::IMPACT_NITPICK,
            color: theme.accent,
        },
    }
}
