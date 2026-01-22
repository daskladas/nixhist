//! User Interface layer
//!
//! This module contains all UI-related code:
//! - Theme definitions and colors
//! - Reusable widgets
//! - Tab-specific views
//! - Main render loop

pub mod theme;
pub mod render;
pub mod widgets;

pub use theme::Theme;
pub use render::render;
