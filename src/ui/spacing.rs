//! Standardized spacing constants for consistent UI in LaReview.
//!
//! This module defines a consistent spacing scale to maintain visual consistency
//! across all UI components in the application.
//!
//! All constants are f32 by default. Use .into() or as i8 when i8 values are needed
//! for margin APIs.

/// Extra small spacing value (4px)
pub const SPACING_XS: f32 = 4.0;

/// Small spacing value (8px)
pub const SPACING_SM: f32 = 8.0;

/// Medium spacing value (12px)
pub const SPACING_MD: f32 = 12.0;

/// Large spacing value (16px)
pub const SPACING_LG: f32 = 16.0;

/// Extra large spacing value (24px)
pub const SPACING_XL: f32 = 24.0;

/// Button padding (8px horizontal, 4px vertical) - for temporary UI changes
pub const BUTTON_PADDING: (f32, f32) = (8.0, 4.0);

/// Standard item spacing (8px horizontal, 6px vertical) - for temporary UI changes
pub const ITEM_SPACING: (f32, f32) = (8.0, 6.0);

/// Diff component spacing (0px horizontal, 2px vertical) - for temporary UI changes
#[allow(dead_code)]
pub const DIFF_ITEM_SPACING: (f32, f32) = (0.0, 2.0);

/// Tight item spacing (4px horizontal, 4px vertical) - for temporary UI changes
pub const TIGHT_ITEM_SPACING: (f32, f32) = (4.0, 4.0);
