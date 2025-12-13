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
            bg_primary: MOCHA.base,   // Main content area
            text_primary: MOCHA.text, // Contrasts well with base

            bg_secondary: MOCHA.surface0, // Cards and panels
            text_secondary: MOCHA.text,   // Contrasts well with surface0

            bg_tertiary: MOCHA.crust,  // Subtle backgrounds
            text_tertiary: MOCHA.text, // Contrasts well with crust

            bg_surface: MOCHA.mantle, // Dialogs and dropdowns
            text_surface: MOCHA.text, // Contrasts well with mantle

            bg_muted: MOCHA.surface2, // Muted surfaces
            text_muted: MOCHA.text,   // Contrasts well with surface2

            bg_card: MOCHA.surface1, // Individual cards
            text_card: MOCHA.text,   // Contrasts well with surface1

            // Brand colors
            brand: MOCHA.mauve,   // Primary brand color
            brand_fg: MOCHA.text, // Use text color for better contrast

            // Status colors
            success: MOCHA.green,   // Success states
            success_fg: MOCHA.text, // Contrasts well with green

            warning: MOCHA.yellow,  // Warning states
            warning_fg: MOCHA.text, // Contrasts well with yellow

            destructive: MOCHA.red,     // Error states
            destructive_fg: MOCHA.text, // Contrasts well with red

            accent: MOCHA.blue,    // Secondary accent color
            accent_fg: MOCHA.text, // Contrasts well with blue

            // Text hierarchy
            text_inverse: egui::Color32::from_rgb(255, 255, 255), // White for dark backgrounds
            text_disabled: MOCHA.overlay2,                        // Subtle disabled text
            text_on_muted: MOCHA.text,                            // Text on muted backgrounds

            // Border colors
            border: MOCHA.surface2,           // Primary borders
            border_secondary: MOCHA.surface1, // Secondary borders

            // Special colors
            transparent: egui::Color32::TRANSPARENT,

            interactive_selected: MOCHA.sky, // For selected states

            text_accent: MOCHA.lavender, // Special accent text
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
