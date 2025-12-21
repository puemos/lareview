use crate::domain::{ThreadImpact, ThreadStatus};
use crate::ui::theme::Theme;
use egui_phosphor::regular as icons;

pub struct Visuals {
    pub icon: &'static str,
    pub label: &'static str,
    pub color: eframe::egui::Color32,
}

pub fn status_visuals(status: ThreadStatus, theme: &Theme) -> Visuals {
    match status {
        ThreadStatus::Todo => Visuals {
            icon: icons::CIRCLE_DASHED,
            label: "Todo",
            color: theme.brand,
        },
        ThreadStatus::Wip => Visuals {
            icon: icons::CIRCLE_NOTCH,
            label: "WIP",
            color: theme.warning,
        },
        ThreadStatus::Done => Visuals {
            icon: icons::CHECK_CIRCLE,
            label: "Done",
            color: theme.success,
        },
        ThreadStatus::Reject => Visuals {
            icon: icons::PROHIBIT,
            label: "Reject",
            color: theme.destructive,
        },
    }
}

pub fn impact_visuals(impact: ThreadImpact, theme: &Theme) -> Visuals {
    match impact {
        ThreadImpact::Blocking => Visuals {
            icon: icons::WARNING_CIRCLE,
            label: "Blocking",
            color: theme.destructive,
        },
        ThreadImpact::NiceToHave => Visuals {
            icon: icons::HAND_HEART,
            label: "Nice to have",
            color: theme.accent,
        },
        ThreadImpact::Nitpick => Visuals {
            icon: icons::PENCIL_SIMPLE_LINE,
            label: "Nitpick",
            color: theme.text_muted,
        },
    }
}
