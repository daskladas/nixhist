//! Reusable UI widgets
//!
//! Contains common UI components used across multiple tabs:
//! - Popup dialogs (confirmation, error)
//! - Progress indicators
//! - Custom list rendering

use crate::ui::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Render a centered popup dialog
pub fn render_popup(
    frame: &mut Frame,
    title: &str,
    content: Vec<Line>,
    buttons: &[(&str, char)], // (label, key)
    theme: &Theme,
    area: Rect,
) {
    // Calculate popup size
    let popup_width = 56.min(area.width.saturating_sub(4));
    let popup_height = (content.len() as u16 + 8).min(area.height.saturating_sub(4));

    let popup_area = centered_rect(popup_width, popup_height, area);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    // Render popup background and border
    let block = Block::default()
        .title(format!(" {} ", title))
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .style(theme.text());

    frame.render_widget(block, popup_area);

    // Inner area for content
    let inner = Rect {
        x: popup_area.x + 2,
        y: popup_area.y + 2,
        width: popup_area.width.saturating_sub(4),
        height: popup_area.height.saturating_sub(5),
    };

    // Render content
    let content_widget = Paragraph::new(content)
        .style(theme.text())
        .wrap(Wrap { trim: false });
    frame.render_widget(content_widget, inner);

    // Render buttons at bottom
    if !buttons.is_empty() {
        let button_area = Rect {
            x: popup_area.x + 2,
            y: popup_area.y + popup_area.height - 3,
            width: popup_area.width.saturating_sub(4),
            height: 1,
        };

        let button_spans: Vec<Span> = buttons
            .iter()
            .enumerate()
            .flat_map(|(i, (label, key))| {
                let mut spans = vec![
                    Span::styled("[", theme.text_dim()),
                    Span::styled(
                        key.to_string(),
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("] ", theme.text_dim()),
                    Span::styled(*label, theme.text()),
                ];
                if i < buttons.len() - 1 {
                    spans.push(Span::raw("    "));
                }
                spans
            })
            .collect();

        let buttons_widget = Paragraph::new(Line::from(button_spans))
            .alignment(Alignment::Center);
        frame.render_widget(buttons_widget, button_area);
    }
}

/// Render a confirmation popup with Yes/No buttons
pub fn render_confirm_popup(
    frame: &mut Frame,
    title: &str,
    message: &str,
    command_preview: Option<&str>,
    theme: &Theme,
    area: Rect,
) {
    let mut content = vec![
        Line::raw(""),
        Line::raw(message),
        Line::raw(""),
    ];

    if let Some(cmd) = command_preview {
        content.push(Line::styled("Command to execute:", theme.text_dim()));
        content.push(Line::raw(""));
        content.push(Line::styled(cmd, Style::default().fg(theme.fg_dim)));
        content.push(Line::raw(""));
    }

    render_popup(
        frame,
        title,
        content,
        &[("Yes", 'y'), ("Cancel", 'n')],
        theme,
        area,
    );
}

/// Render an error popup
pub fn render_error_popup(
    frame: &mut Frame,
    title: &str,
    message: &str,
    theme: &Theme,
    area: Rect,
) {
    let content = vec![
        Line::raw(""),
        Line::styled(message, theme.error()),
        Line::raw(""),
    ];

    render_popup(
        frame,
        title,
        content,
        &[("OK", 'o')],
        theme,
        area,
    );
}

/// Render an undo countdown popup
pub fn render_undo_popup(
    frame: &mut Frame,
    message: &str,
    seconds_remaining: u8,
    theme: &Theme,
    area: Rect,
) {
    // Progress bar
    let total_width = 30;
    let filled = (seconds_remaining as usize * total_width / 10).min(total_width);
    let empty = total_width - filled;
    let progress_bar = format!(
        "{}{}",
        "█".repeat(filled),
        "░".repeat(empty)
    );

    let content = vec![
        Line::raw(""),
        Line::styled("⚠  Last chance to undo!", theme.warning()),
        Line::raw(""),
        Line::raw(message),
        Line::raw(""),
        Line::from(vec![
            Span::styled(progress_bar, theme.warning()),
            Span::raw(format!("  {}s", seconds_remaining)),
        ]),
        Line::raw(""),
    ];

    render_popup(
        frame,
        "Undo Available",
        content,
        &[("Undo", 'u'), ("Confirm", '\x1b')], // Esc for confirm
        theme,
        area,
    );
}

/// Render a loading indicator
pub fn render_loading(
    frame: &mut Frame,
    message: &str,
    theme: &Theme,
    area: Rect,
) {
    let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let frame_idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() / 100) as usize % spinner_frames.len();

    let content = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled(spinner_frames[frame_idx], Style::default().fg(theme.accent)),
            Span::raw(" "),
            Span::styled(message, theme.text()),
        ]),
        Line::raw(""),
    ];

    let popup_width = 40.min(area.width.saturating_sub(4));
    let popup_area = centered_rect(popup_width, 5, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
        .style(theme.text());

    frame.render_widget(block, popup_area);

    let inner = Rect {
        x: popup_area.x + 2,
        y: popup_area.y + 1,
        width: popup_area.width.saturating_sub(4),
        height: 3,
    };

    let loading = Paragraph::new(content)
        .alignment(Alignment::Center);
    frame.render_widget(loading, inner);
}

/// Render a success flash message (bottom of screen)
pub fn render_flash_message(
    frame: &mut Frame,
    message: &str,
    is_error: bool,
    theme: &Theme,
    area: Rect,
) {
    let style = if is_error { theme.error() } else { theme.success() };
    let prefix = if is_error { "✗ " } else { "✓ " };

    let flash_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };

    let flash = Paragraph::new(Line::from(vec![
        Span::styled(prefix, style),
        Span::styled(message, style),
    ]));

    frame.render_widget(flash, flash_area);
}

/// Render status bar at bottom
pub fn render_status_bar(
    frame: &mut Frame,
    left_content: &str,
    right_content: &str,
    theme: &Theme,
    area: Rect,
) {
    let status_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };

    // Clear the line first
    frame.render_widget(Clear, status_area);

    // Left side
    let left_widget = Paragraph::new(left_content)
        .style(theme.text_dim());
    
    // Right side
    let right_len = right_content.len() as u16;
    let right_area = Rect {
        x: status_area.x + status_area.width.saturating_sub(right_len + 1),
        y: status_area.y,
        width: right_len + 1,
        height: 1,
    };
    let right_widget = Paragraph::new(right_content)
        .style(theme.text_dim());

    frame.render_widget(left_widget, status_area);
    frame.render_widget(right_widget, right_area);
}

/// Helper: Create a centered rect of given size
pub fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect { x, y, width, height }
}

/// Helper: Create horizontal layout with given percentages
pub fn horizontal_split(area: Rect, percentages: &[u16]) -> Vec<Rect> {
    let constraints: Vec<Constraint> = percentages
        .iter()
        .map(|p| Constraint::Percentage(*p))
        .collect();
    
    Layout::horizontal(constraints).split(area).to_vec()
}

/// Helper: Create vertical layout with given constraints
pub fn vertical_layout(area: Rect, constraints: Vec<Constraint>) -> Vec<Rect> {
    Layout::vertical(constraints).split(area).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_centered_rect() {
        let area = Rect::new(0, 0, 100, 50);
        let popup = centered_rect(40, 20, area);
        
        assert_eq!(popup.x, 30);
        assert_eq!(popup.y, 15);
        assert_eq!(popup.width, 40);
        assert_eq!(popup.height, 20);
    }

    #[test]
    fn test_horizontal_split() {
        let area = Rect::new(0, 0, 100, 50);
        let splits = horizontal_split(area, &[50, 50]);
        
        assert_eq!(splits.len(), 2);
        assert_eq!(splits[0].width, 50);
        assert_eq!(splits[1].width, 50);
    }
}
