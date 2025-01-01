use crate::domain::{FeedbackImpact, ReviewStatus, RiskLevel};
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
            color: theme.status_todo,
        },
        ReviewStatus::InProgress => Visuals {
            label: "In Progress",
            icon: icons::STATUS_IN_PROGRESS,
            color: theme.status_in_progress,
        },
        ReviewStatus::Done => Visuals {
            label: "Done",
            icon: icons::STATUS_DONE,
            color: theme.status_done,
        },
        ReviewStatus::Ignored => Visuals {
            label: "Ignored",
            icon: icons::STATUS_IGNORED,
            color: theme.status_ignored,
        },
    }
}

pub fn impact_visuals(impact: FeedbackImpact, theme: &Theme) -> Visuals {
    match impact {
        FeedbackImpact::Blocking => Visuals {
            label: "Blocking",
            icon: icons::IMPACT_BLOCKING,
            color: theme.impact_blocking,
        },
        FeedbackImpact::NiceToHave => Visuals {
            label: "Nice to have",
            icon: icons::IMPACT_NICE_TO_HAVE,
            color: theme.impact_nice_to_have,
        },
        FeedbackImpact::Nitpick => Visuals {
            label: "Nitpick",
            icon: icons::IMPACT_NITPICK,
            color: theme.impact_nitpick,
        },
    }
}

pub fn risk_visuals(risk: RiskLevel, theme: &Theme) -> Visuals {
    match risk {
        RiskLevel::High => Visuals {
            label: "High risk",
            icon: icons::RISK_HIGH,
            color: theme.risk_high,
        },
        RiskLevel::Medium => Visuals {
            label: "Med risk",
            icon: icons::RISK_MEDIUM,
            color: theme.risk_medium,
        },
        RiskLevel::Low => Visuals {
            label: "Low risk",
            icon: icons::RISK_LOW,
            color: theme.risk_low,
        },
    }
}
