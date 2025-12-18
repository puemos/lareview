#![allow(dead_code)] // Allow unused fields - comprehensive theme definition for consistency

//! Theme definitions for LaReview
//!
//! This module provides semantic color names built on top of the Catppuccin Mocha palette
//! following shadcn/ui best practices for consistent and accessible theming throughout the application.

use catppuccin_egui::MOCHA;
use eframe::egui;

/// Semantic color theme that builds upon the Catppuccin Mocha palette following shadcn/ui best practices
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    // Background colors with matching text colors for accessibility
    /// Primary background - main application surface
    pub bg_primary: egui::Color32,
    /// Primary text color that contrasts well with bg_primary
    pub text_primary: egui::Color32,

    /// Secondary background - cards, panels, elevated surfaces
    pub bg_secondary: egui::Color32,
    /// Text color that contrasts well with bg_secondary
    pub text_secondary: egui::Color32,

    /// Tertiary background - subtle surfaces
    pub bg_tertiary: egui::Color32,
    /// Text color that contrasts well with bg_tertiary
    pub text_tertiary: egui::Color32,

    /// Surface background - dialogs, dropdowns
    pub bg_surface: egui::Color32,
    /// Text color that contrasts well with bg_surface
    pub text_surface: egui::Color32,

    /// Muted background - for less prominent elements
    pub bg_muted: egui::Color32,
    /// Text color that contrasts well with bg_muted
    pub text_muted: egui::Color32,

    /// Card background - for individual cards
    pub bg_card: egui::Color32,
    /// Text color that contrasts well with bg_card
    pub text_card: egui::Color32,

    // Brand colors
    /// Primary brand color
    pub brand: egui::Color32,
    /// Brand foreground color (text/icon on brand backgrounds)
    pub brand_fg: egui::Color32,

    // Semantic status colors
    /// Success state color
    pub success: egui::Color32,
    /// Success foreground color
    pub success_fg: egui::Color32,

    /// Warning state color
    pub warning: egui::Color32,
    /// Warning foreground color
    pub warning_fg: egui::Color32,

    /// Destructive/error state color
    pub destructive: egui::Color32,
    /// Destructive foreground color
    pub destructive_fg: egui::Color32,

    /// Accent color (secondary brand color)
    pub accent: egui::Color32,
    /// Accent foreground color
    pub accent_fg: egui::Color32,

    // Text hierarchy
    /// Text for inverse situations (dark bg, light text)
    pub text_inverse: egui::Color32,
    /// Disabled text color
    pub text_disabled: egui::Color32,
    /// Muted text color (less important than primary)
    pub text_on_muted: egui::Color32,

    // Border colors
    /// Primary border color
    pub border: egui::Color32,
    /// Secondary border color
    pub border_secondary: egui::Color32,

    // Special colors
    /// Transparent color
    pub transparent: egui::Color32,

    /// Color for selected interactive elements
    pub interactive_selected: egui::Color32,

    /// Accent text color
    pub text_accent: egui::Color32,
}

impl Theme {
    /// Creates a new theme based on the Catppuccin Mocha palette following shadcn/ui best practices
    pub fn mocha() -> Self {
        Self {
            // Background-text pairs for accessibility
            bg_primary: MOCHA.crust, // Main application backdrop (Extreme Dark)
            text_primary: MOCHA.text,

            bg_secondary: MOCHA.mantle, // Sidebars, Header
            text_secondary: MOCHA.text,

            bg_tertiary: MOCHA.base, // Panels, Cards sitting on secondary surfaces
            text_tertiary: MOCHA.text,

            bg_surface: MOCHA.base, // Dialogs and dropdowns
            text_surface: MOCHA.text,

            bg_muted: MOCHA.surface0, // Muted surfaces (Gray)
            text_muted: MOCHA.subtext0,

            bg_card: MOCHA.base, // Individual cards
            text_card: MOCHA.text,

            // Brand colors
            brand: MOCHA.mauve,    // Primary terminal brand color
            brand_fg: MOCHA.crust, // Dark text on light brand background

            // Status colors
            success: MOCHA.green,
            success_fg: MOCHA.crust,

            warning: MOCHA.yellow,
            warning_fg: MOCHA.crust,

            destructive: MOCHA.red,
            destructive_fg: MOCHA.crust,

            accent: MOCHA.blue,
            accent_fg: MOCHA.crust,

            // Text hierarchy
            text_inverse: egui::Color32::from_rgb(255, 255, 255),
            text_disabled: MOCHA.overlay2,
            text_on_muted: MOCHA.subtext1,

            // Border colors
            border: MOCHA.overlay0,           // Strong TUI-style borders
            border_secondary: MOCHA.surface1, // Subtle borders

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
