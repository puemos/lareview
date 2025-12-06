#![allow(dead_code)]
//! Theme - "Sharp & Stone" design system

use gpui::{Hsla, hsla};

/// Color palette for the Sharp & Stone theme
pub struct ThemeColors {
    // Backgrounds
    pub bg: Hsla,
    pub surface: Hsla,
    pub surface_alt: Hsla,

    // Borders
    pub border: Hsla,
    pub border_strong: Hsla,

    // Text
    pub text: Hsla,
    pub text_strong: Hsla,
    pub text_muted: Hsla,

    // Primary
    pub primary: Hsla,
    pub primary_contrast: Hsla,

    // Semantic
    pub success: Hsla,
    pub warning: Hsla,
    pub danger: Hsla,
}

impl Default for ThemeColors {
    fn default() -> Self {
        let hue_stone = 215.0 / 360.0;

        Self {
            // Backgrounds
            bg: hsla(hue_stone, 0.20, 0.96, 1.0),
            surface: hsla(0.0, 0.0, 1.0, 1.0),
            surface_alt: hsla(hue_stone, 0.20, 0.94, 1.0),

            // Borders
            border: hsla(hue_stone, 0.15, 0.85, 1.0),
            border_strong: hsla(hue_stone, 0.15, 0.70, 1.0),

            // Text
            text: hsla(hue_stone, 0.30, 0.15, 1.0),
            text_strong: hsla(hue_stone, 0.40, 0.10, 1.0),
            text_muted: hsla(hue_stone, 0.15, 0.45, 1.0),

            // Primary - Technical Blue
            primary: hsla(220.0 / 360.0, 0.90, 0.50, 1.0),
            primary_contrast: hsla(0.0, 0.0, 1.0, 1.0),

            // Semantic
            success: hsla(150.0 / 360.0, 0.80, 0.35, 1.0),
            warning: hsla(40.0 / 360.0, 0.90, 0.40, 1.0),
            danger: hsla(350.0 / 360.0, 0.80, 0.50, 1.0),
        }
    }
}

/// Spacing scale
pub struct Spacing {
    pub space_1: f32,
    pub space_2: f32,
    pub space_3: f32,
    pub space_4: f32,
    pub space_5: f32,
    pub space_6: f32,
    pub space_8: f32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            space_1: 4.0,
            space_2: 8.0,
            space_3: 12.0,
            space_4: 16.0,
            space_5: 20.0,
            space_6: 24.0,
            space_8: 32.0,
        }
    }
}

/// Complete theme
#[derive(Default)]
pub struct Theme {
    pub colors: ThemeColors,
    pub spacing: Spacing,
}

/// Global theme instance
pub fn theme() -> Theme {
    Theme::default()
}
