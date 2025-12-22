use crate::domain::{ReviewStatus, ThreadImpact};
use crate::ui::icons;
use crate::ui::theme::Theme;
use eframe::egui;

pub struct Visuals {
    pub label: &'static str,
    pub icon: &'static str,
    pub color: egui::Color32,
}

pub fn status_visuals(status: ReviewStatus, theme: &Theme) -> Visuals {
    match status {
        ReviewStatus::Todo => Visuals {
            label: "To Do",
            icon: icons::STATUS_TODO,
            color: theme.brand,
        },
        ReviewStatus::InProgress => Visuals {
            label: "In Progress",
            icon: icons::STATUS_WIP,
            color: theme.accent,
        },
        ReviewStatus::Done => Visuals {
            label: "Done",
            icon: icons::STATUS_DONE,
            color: theme.success,
        },
        ReviewStatus::Ignored => Visuals {
            label: "Ignored",
            icon: icons::STATUS_IGNORED,
            color: theme.text_muted,
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
