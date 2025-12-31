//! Theme definitions for LaReview
//!
//! This module provides semantic color names built on top of the Catppuccin Mocha palette
//! for consistent and accessible theming throughout the application.

use catppuccin_egui::MOCHA;
use eframe::egui;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub bg_primary: egui::Color32,
    pub text_primary: egui::Color32,

    pub bg_secondary: egui::Color32,
    pub text_secondary: egui::Color32,

    pub bg_tertiary: egui::Color32,
    pub text_tertiary: egui::Color32,

    pub bg_surface: egui::Color32,
    pub text_surface: egui::Color32,

    pub bg_muted: egui::Color32,
    pub text_muted: egui::Color32,

    pub bg_card: egui::Color32,
    pub text_card: egui::Color32,

    pub brand: egui::Color32,
    pub brand_fg: egui::Color32,

    pub status_todo: egui::Color32,
    pub status_in_progress: egui::Color32,
    pub status_done: egui::Color32,
    pub status_ignored: egui::Color32,

    pub impact_nitpick: egui::Color32,
    pub impact_nice_to_have: egui::Color32,
    pub impact_blocking: egui::Color32,

    pub risk_low: egui::Color32,
    pub risk_medium: egui::Color32,
    pub risk_high: egui::Color32,

    pub success: egui::Color32,
    pub success_fg: egui::Color32,

    pub warning: egui::Color32,
    pub warning_fg: egui::Color32,

    pub destructive: egui::Color32,
    pub destructive_fg: egui::Color32,

    pub accent: egui::Color32,
    pub accent_fg: egui::Color32,

    pub text_inverse: egui::Color32,
    pub text_disabled: egui::Color32,
    pub text_on_muted: egui::Color32,

    pub border: egui::Color32,
    pub border_secondary: egui::Color32,

    pub transparent: egui::Color32,

    pub interactive_selected: egui::Color32,

    pub text_accent: egui::Color32,
}

impl Theme {
    /// Creates a new theme based on the Catppuccin Mocha palette
    pub fn mocha() -> Self {
        Self {
            // Background-text pairs for accessibility
            bg_primary: MOCHA.base, // Standard dark background (Lighter than crust)
            text_primary: egui::Color32::from_rgb(230, 233, 239), // Brighter white (Mocha text is slightly muted)

            bg_secondary: MOCHA.mantle, // Elevated surfaces (Sidebars, Header)
            text_secondary: egui::Color32::from_rgb(186, 194, 222),

            bg_tertiary: MOCHA.surface0, // Elevated surfaces (Cards, focus areas)
            text_tertiary: egui::Color32::from_rgb(166, 173, 200),

            bg_surface: MOCHA.mantle, // Dialogs and dropdowns
            text_surface: egui::Color32::from_rgb(230, 233, 239),

            bg_muted: MOCHA.surface0, // Muted surfaces
            text_muted: MOCHA.subtext0,

            bg_card: MOCHA.mantle, // Individual cards
            text_card: egui::Color32::from_rgb(230, 233, 239),

            // Brand colors
            brand: MOCHA.mauve, // Primary terminal brand color
            brand_fg: MOCHA.base,

            // --- Standardized Enum Colors ---

            // ReviewStatus
            status_todo: MOCHA.subtext0,
            status_in_progress: MOCHA.yellow,
            status_done: MOCHA.green,
            status_ignored: MOCHA.red,

            // FeedbackImpact
            impact_nitpick: MOCHA.blue,
            impact_nice_to_have: MOCHA.yellow,
            impact_blocking: MOCHA.red,

            // RiskLevel
            risk_low: MOCHA.blue,
            risk_medium: MOCHA.yellow,
            risk_high: MOCHA.red,

            // Status colors
            success: MOCHA.green,
            success_fg: MOCHA.base,

            warning: MOCHA.yellow,
            warning_fg: MOCHA.base,

            destructive: MOCHA.red,
            destructive_fg: MOCHA.base,

            accent: MOCHA.blue,
            accent_fg: MOCHA.base,

            // Text hierarchy
            text_inverse: egui::Color32::from_rgb(255, 255, 255),
            text_disabled: MOCHA.overlay1,
            text_on_muted: MOCHA.subtext1,

            // Border colors
            border: MOCHA.surface1,           // Subtle borders
            border_secondary: MOCHA.surface0, // Very subtle borders

            // Special colors
            transparent: egui::Color32::TRANSPARENT,

            interactive_selected: MOCHA.sky,

            text_accent: MOCHA.lavender,
        }
    }

    /// Gets the theme based on current application settings
    /// Currently just returns the Mocha theme but could be extended for theme switching
    pub fn current() -> Self {
        Self::mocha()
    }
}

/// Global instance of the current theme
/// This can be accessed throughout the application for consistent theming
pub fn current_theme() -> Theme {
    Theme::current()
}

/// Module with common color utilities
pub mod colors {
    /// Transparent color constant
    pub const TRANSPARENT: egui::Color32 = egui::Color32::TRANSPARENT;
}
