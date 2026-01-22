//! Theme definitions for nixhist
//!
//! Provides three built-in themes: Gruvbox, Nord, and Transparent.
//! Each theme defines colors for all UI elements.

use crate::config::ThemeName;
use ratatui::style::{Color, Modifier, Style};

/// Complete theme with all required colors
#[derive(Debug, Clone)]
pub struct Theme {
    // Base colors
    pub bg: Color,
    pub fg: Color,
    pub fg_dim: Color,
    
    // Accent colors
    pub accent: Color,
    pub accent_dim: Color,
    
    // Status colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    
    // UI element colors
    pub border: Color,
    pub border_focused: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    
    // Diff colors
    pub diff_added: Color,
    pub diff_removed: Color,
    pub diff_updated: Color,
    
    // Special indicators
    pub current_marker: Color,
    pub pinned_marker: Color,
    pub boot_marker: Color,
}

impl Theme {
    /// Create a theme from a theme name
    pub fn from_name(name: ThemeName) -> Self {
        match name {
            ThemeName::Gruvbox => Self::gruvbox(),
            ThemeName::Nord => Self::nord(),
            ThemeName::Transparent => Self::transparent(),
        }
    }

    /// Gruvbox dark theme (default)
    pub fn gruvbox() -> Self {
        Self {
            // Base
            bg: Color::Rgb(40, 40, 40),           // #282828
            fg: Color::Rgb(235, 219, 178),        // #ebdbb2
            fg_dim: Color::Rgb(146, 131, 116),    // #928374
            
            // Accent (orange)
            accent: Color::Rgb(254, 128, 25),     // #fe8019
            accent_dim: Color::Rgb(214, 93, 14),  // #d65d0e
            
            // Status
            success: Color::Rgb(184, 187, 38),    // #b8bb26
            warning: Color::Rgb(250, 189, 47),    // #fabd2f
            error: Color::Rgb(251, 73, 52),       // #fb4934
            
            // UI elements
            border: Color::Rgb(80, 73, 69),       // #504945
            border_focused: Color::Rgb(168, 153, 132), // #a89984
            selection_bg: Color::Rgb(80, 73, 69), // #504945
            selection_fg: Color::Rgb(235, 219, 178), // #ebdbb2
            
            // Diff
            diff_added: Color::Rgb(184, 187, 38),    // #b8bb26 (green)
            diff_removed: Color::Rgb(251, 73, 52),   // #fb4934 (red)
            diff_updated: Color::Rgb(131, 165, 152), // #83a598 (blue)
            
            // Markers
            current_marker: Color::Rgb(184, 187, 38),   // green
            pinned_marker: Color::Rgb(250, 189, 47),    // yellow
            boot_marker: Color::Rgb(131, 165, 152),     // blue
        }
    }

    /// Nord theme
    pub fn nord() -> Self {
        Self {
            // Base (Polar Night)
            bg: Color::Rgb(46, 52, 64),           // #2e3440
            fg: Color::Rgb(236, 239, 244),        // #eceff4
            fg_dim: Color::Rgb(76, 86, 106),      // #4c566a
            
            // Accent (Frost - blue)
            accent: Color::Rgb(136, 192, 208),    // #88c0d0
            accent_dim: Color::Rgb(94, 129, 172), // #5e81ac
            
            // Status (Aurora)
            success: Color::Rgb(163, 190, 140),   // #a3be8c (green)
            warning: Color::Rgb(235, 203, 139),   // #ebcb8b (yellow)
            error: Color::Rgb(191, 97, 106),      // #bf616a (red)
            
            // UI elements
            border: Color::Rgb(59, 66, 82),       // #3b4252
            border_focused: Color::Rgb(136, 192, 208), // #88c0d0
            selection_bg: Color::Rgb(76, 86, 106),    // #4c566a
            selection_fg: Color::Rgb(236, 239, 244),  // #eceff4
            
            // Diff
            diff_added: Color::Rgb(163, 190, 140),   // green
            diff_removed: Color::Rgb(191, 97, 106),  // red
            diff_updated: Color::Rgb(129, 161, 193), // blue
            
            // Markers
            current_marker: Color::Rgb(163, 190, 140),
            pinned_marker: Color::Rgb(235, 203, 139),
            boot_marker: Color::Rgb(136, 192, 208),
        }
    }

    /// Transparent theme (uses terminal colors)
    pub fn transparent() -> Self {
        Self {
            // Base - use terminal defaults
            bg: Color::Reset,
            fg: Color::Reset,
            fg_dim: Color::DarkGray,
            
            // Accent
            accent: Color::Cyan,
            accent_dim: Color::Blue,
            
            // Status
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            
            // UI elements
            border: Color::DarkGray,
            border_focused: Color::Cyan,
            selection_bg: Color::DarkGray,
            selection_fg: Color::White,
            
            // Diff
            diff_added: Color::Green,
            diff_removed: Color::Red,
            diff_updated: Color::Blue,
            
            // Markers
            current_marker: Color::Green,
            pinned_marker: Color::Yellow,
            boot_marker: Color::Cyan,
        }
    }

    // Style helpers for common UI patterns

    /// Default text style
    pub fn text(&self) -> Style {
        Style::default().fg(self.fg).bg(self.bg)
    }

    /// Dimmed text style
    pub fn text_dim(&self) -> Style {
        Style::default().fg(self.fg_dim).bg(self.bg)
    }

    /// Title/header style
    pub fn title(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .bg(self.bg)
            .add_modifier(Modifier::BOLD)
    }

    /// Selected item style
    pub fn selected(&self) -> Style {
        Style::default()
            .fg(self.selection_fg)
            .bg(self.selection_bg)
            .add_modifier(Modifier::BOLD)
    }

    /// Border style (unfocused)
    pub fn border(&self) -> Style {
        Style::default().fg(self.border).bg(self.bg)
    }

    /// Border style (focused)
    pub fn border_focused(&self) -> Style {
        Style::default().fg(self.border_focused).bg(self.bg)
    }

    /// Tab style (inactive)
    pub fn tab_inactive(&self) -> Style {
        Style::default().fg(self.fg_dim).bg(self.bg)
    }

    /// Tab style (active)
    pub fn tab_active(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .bg(self.bg)
            .add_modifier(Modifier::BOLD)
    }

    /// Success message style
    pub fn success(&self) -> Style {
        Style::default().fg(self.success).bg(self.bg)
    }

    /// Warning message style
    pub fn warning(&self) -> Style {
        Style::default().fg(self.warning).bg(self.bg)
    }

    /// Error message style
    pub fn error(&self) -> Style {
        Style::default().fg(self.error).bg(self.bg)
    }

    /// Added item in diff
    pub fn diff_added(&self) -> Style {
        Style::default().fg(self.diff_added).bg(self.bg)
    }

    /// Removed item in diff
    pub fn diff_removed(&self) -> Style {
        Style::default().fg(self.diff_removed).bg(self.bg)
    }

    /// Updated item in diff
    pub fn diff_updated(&self) -> Style {
        Style::default().fg(self.diff_updated).bg(self.bg)
    }

    /// Current generation marker
    pub fn marker_current(&self) -> Style {
        Style::default()
            .fg(self.current_marker)
            .add_modifier(Modifier::BOLD)
    }

    /// Pinned generation marker
    pub fn marker_pinned(&self) -> Style {
        Style::default().fg(self.pinned_marker)
    }

    /// Boot entry marker
    pub fn marker_boot(&self) -> Style {
        Style::default().fg(self.boot_marker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_from_name() {
        let gruvbox = Theme::from_name(ThemeName::Gruvbox);
        assert_eq!(gruvbox.bg, Color::Rgb(40, 40, 40));

        let nord = Theme::from_name(ThemeName::Nord);
        assert_eq!(nord.bg, Color::Rgb(46, 52, 64));

        let transparent = Theme::from_name(ThemeName::Transparent);
        assert_eq!(transparent.bg, Color::Reset);
    }
}
