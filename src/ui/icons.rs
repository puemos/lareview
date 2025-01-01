//! Centralized icon registry for LaReview.
//!
//! All UI components should use these constants instead of directly
//! referencing `egui_phosphor` to ensure visual consistency.

use egui_phosphor::regular::{self as icons};

// --- Task & Feedback Status ---

pub const STATUS_TODO: &str = icons::CIRCLE;
pub const STATUS_IN_PROGRESS: &str = icons::CIRCLE_DASHED;
pub const STATUS_DONE: &str = icons::CHECK_CIRCLE;
pub const STATUS_IGNORED: &str = icons::X_CIRCLE;

// --- Risk Levels ---
pub const RISK_LOW: &str = icons::CARET_CIRCLE_DOWN;
pub const RISK_MEDIUM: &str = icons::CARET_CIRCLE_UP;
pub const RISK_HIGH: &str = icons::CARET_CIRCLE_DOUBLE_UP;

// --- Navigation & Views ---
pub const VIEW_GENERATE: &str = icons::ASTERISK;
pub const VIEW_REVIEW: &str = icons::EYES;
pub const VIEW_REPOS: &str = icons::FOLDER;
pub const VIEW_SETTINGS: &str = icons::GEAR;

pub const TAB_DESCRIPTION: &str = icons::FILE_TEXT;
pub const TAB_DIAGRAM: &str = icons::CHART_BAR;
pub const TAB_CHANGES: &str = icons::GIT_DIFF;
pub const TAB_FEEDBACK: &str = icons::CHAT_CIRCLE;

// --- Common Actions ---
pub const ACTION_RUN: &str = icons::PLAY;
pub const ACTION_STOP: &str = icons::STOP;
pub const ACTION_DELETE: &str = icons::TRASH_SIMPLE;
pub const ACTION_TRASH: &str = icons::TRASH;
pub const ACTION_EXPORT: &str = icons::EXPORT;
pub const ACTION_REPLY: &str = icons::PAPER_PLANE_RIGHT;
pub const ACTION_CLOSE: &str = icons::X;
pub const ACTION_CLEAR: &str = icons::TRASH;
pub const ACTION_REFRESH: &str = icons::ARROW_CLOCKWISE;
pub const ACTION_OPEN_WINDOW: &str = icons::ARROW_SQUARE_OUT;
pub const ACTION_EXPAND: &str = icons::ARROWS_OUT_SIMPLE;
pub const ACTION_COLLAPSE: &str = icons::ARROWS_IN_SIMPLE;
pub const ACTION_BACK: &str = icons::ARROW_SQUARE_IN;
pub const ACTION_COPY: &str = icons::COPY;
pub const ACTION_SAVE: &str = icons::FLOPPY_DISK;

// --- Symbols ---
pub const ICON_PLAN: &str = icons::LIST_CHECKS;
pub const ICON_GITHUB: &str = icons::GITHUB_LOGO;
pub const ICON_EMPTY: &str = icons::BOUNDING_BOX;
pub const ICON_CHECK: &str = icons::CHECK_CIRCLE;
pub const ICON_WARNING: &str = icons::WARNING;
pub const ICON_FILES: &str = icons::FILES;
pub const ICON_PLUS: &str = icons::PLUS;
pub const ICON_MINUS: &str = icons::MINUS;
pub const ICON_SQUARE: &str = icons::SQUARE;
pub const ICON_CHECK_SQUARE: &str = icons::CHECK_SQUARE;
pub const ICON_DOT: &str = icons::DOT_OUTLINE;
pub const ICON_ARROW_RIGHT: &str = icons::ARROW_RIGHT;
pub const CHEVRON_DOWN: &str = icons::CARET_DOWN;

// --- Impact ---
pub const IMPACT_BLOCKING: &str = icons::HAND_PALM;
pub const IMPACT_NICE_TO_HAVE: &str = icons::LIGHTBULB;
pub const IMPACT_NITPICK: &str = icons::MICROSCOPE;
