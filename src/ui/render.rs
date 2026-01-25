//! Main rendering module
//!
//! Handles rendering the complete UI including:
//! - Header with hostname and tab bar
//! - Active tab content
//! - Popups and overlays
//! - Status bar

use crate::app::{App, PopupState};
use crate::types::{Generation, GenerationDiff, ProfileType, Tab};
use crate::ui::{theme::Theme, widgets};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Tabs, Wrap},
    Frame,
};

/// Main render function - entry point for all UI rendering
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Main layout: header, content, status bar
    let layout = Layout::vertical([
        Constraint::Length(3),  // Header + tabs
        Constraint::Min(10),    // Content
        Constraint::Length(1),  // Status bar
    ])
    .split(area);

    // Render header with tabs
    render_header(frame, app, layout[0]);

    // Render active tab content
    render_tab_content(frame, app, layout[1]);

    // Render status bar
    render_status_bar(frame, app, layout[2]);

    // Render popup overlays (if any)
    render_popups(frame, app, area);
}

/// Render header with hostname and tab bar
fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    // Header block - FIX: Add background style first
    let header_block = Block::default()
        .style(theme.block_style())
        .title(format!(" nixhist · {} ", app.system_info.hostname))
        .title_style(theme.title())
        .borders(Borders::BOTTOM)
        .border_style(theme.border());

    frame.render_widget(header_block.clone(), area);

    // Tab bar
    let tab_titles: Vec<Line> = Tab::all()
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            let style = if app.active_tab == *tab {
                theme.tab_active()
            } else {
                theme.tab_inactive()
            };
            Line::styled(format!("[{}] {}", i + 1, tab.label()), style)
        })
        .collect();

    let tabs = Tabs::new(tab_titles)
        .select(app.active_tab.index())
        .divider(" │ ")
        .style(theme.text());

    let tabs_area = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: 1,
    };
    frame.render_widget(tabs, tabs_area);
}

/// Render the active tab's content
fn render_tab_content(frame: &mut Frame, app: &App, area: Rect) {
    match app.active_tab {
        Tab::Overview => render_overview_tab(frame, app, area),
        Tab::Packages => render_packages_tab(frame, app, area),
        Tab::Diff => render_diff_tab(frame, app, area),
        Tab::Manage => render_manage_tab(frame, app, area),
        Tab::Settings => render_settings_tab(frame, app, area),
    }
}

/// Render status bar with keybindings
fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    
    let hints = match app.active_tab {
        Tab::Overview => "[j/k] Navigate  [Tab] Switch Panel  [Enter] View Packages  [?] Help  [q] Quit",
        Tab::Packages => "[j/k] Navigate  [/] Filter  [Enter] History  [Esc] Back  [q] Quit",
        Tab::Diff => "[Tab] Switch Dropdown  [j/k] Scroll  [Enter] Select  [q] Quit",
        Tab::Manage => "[Space] Select  [R] Restore  [D] Delete  [P] Pin  [q] Quit",
        Tab::Settings => "[j/k] Navigate  [Enter] Change  [q] Quit",
    };

    widgets::render_status_bar(frame, hints, "", theme, area);
}

/// Render popups if active
fn render_popups(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    match &app.popup {
        PopupState::None => {}
        
        PopupState::Confirm { title, message, command } => {
            widgets::render_confirm_popup(
                frame,
                title,
                message,
                Some(command),
                theme,
                area,
            );
        }
        
        PopupState::Error { title, message } => {
            widgets::render_error_popup(frame, title, message, theme, area);
        }
        
        PopupState::Undo { message, seconds_remaining } => {
            widgets::render_undo_popup(frame, message, *seconds_remaining, theme, area);
        }
        
        PopupState::Loading { message } => {
            widgets::render_loading(frame, message, theme, area);
        }
    }

    // Flash message (success/error feedback)
    if let Some((msg, is_error, _)) = &app.flash_message {
        widgets::render_flash_message(frame, msg, *is_error, theme, area);
    }
}

// === TAB RENDERERS ===

/// Overview tab: System and Home-Manager generations side by side
fn render_overview_tab(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let has_hm = app.home_manager_generations.is_some();

    // Determine layout based on terminal width and config
    let use_side_by_side = has_hm && app.should_use_side_by_side(area.width);

    if use_side_by_side {
        // Split horizontally for System | Home-Manager
        let panels = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

        render_generation_list(
            frame,
            "System",
            &app.system_generations,
            app.overview_system_selected,
            app.overview_focus == 0,
            theme,
            panels[0],
        );

        if let Some(hm_gens) = &app.home_manager_generations {
            render_generation_list(
                frame,
                "Home-Manager",
                hm_gens,
                app.overview_hm_selected,
                app.overview_focus == 1,
                theme,
                panels[1],
            );
        }
    } else {
        // Single panel view
        let title = if app.overview_focus == 0 { "System" } else { "Home-Manager" };
        let gens = if app.overview_focus == 0 {
            &app.system_generations
        } else {
            app.home_manager_generations.as_ref().unwrap_or(&app.system_generations)
        };
        let selected = if app.overview_focus == 0 {
            app.overview_system_selected
        } else {
            app.overview_hm_selected
        };

        render_generation_list(frame, title, gens, selected, true, theme, area);
    }
}

/// Render a list of generations
fn render_generation_list(
    frame: &mut Frame,
    title: &str,
    generations: &[Generation],
    selected: usize,
    is_focused: bool,
    theme: &Theme,
    area: Rect,
) {
    let border_style = if is_focused {
        theme.border_focused()
    } else {
        theme.border()
    };

    // FIX: Add background style first
    let block = Block::default()
        .style(theme.block_style())
        .title(format!(" {} ({}) ", title, generations.len()))
        .title_style(if is_focused { theme.title() } else { theme.text_dim() })
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if generations.is_empty() {
        let empty_msg = Paragraph::new("No generations found")
            .style(theme.text_dim())
            .alignment(Alignment::Center);
        frame.render_widget(empty_msg, inner);
        return;
    }

    // Create list items
    let items: Vec<ListItem> = generations
        .iter()
        .enumerate()
        .map(|(i, gen)| {
            let marker = if gen.is_current {
                "● "
            } else if gen.is_pinned {
                "★ "
            } else {
                "  "
            };

            let marker_style = if gen.is_current {
                theme.marker_current()
            } else if gen.is_pinned {
                theme.marker_pinned()
            } else {
                theme.text()
            };

            let boot_indicator = if gen.in_bootloader { " ⚡" } else { "" };

            let marker_text = marker.to_string();
            let rest_text = format!(
                "#{:<4} {}  {}{}",
                gen.id,
                gen.formatted_date(),
                gen.nixos_version.as_deref().unwrap_or("-"),
                boot_indicator,
            );

            let style = if i == selected {
                theme.selected()
            } else {
                theme.text()
            };

            ListItem::new(Line::from(vec![
                Span::styled(marker_text, marker_style),
                Span::styled(rest_text, style),
            ]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);

    // Show details of selected generation at bottom
    if let Some(gen) = generations.get(selected) {
        let detail_area = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(2),
            width: inner.width,
            height: 2,
        };

        let details = format!(
            "{} · {} · {} pkgs · {}",
            gen.nixos_version.as_deref().unwrap_or("Unknown"),
            gen.kernel_version.as_deref().unwrap_or("-"),
            gen.package_count,
            gen.formatted_size(),
        );

        let detail_widget = Paragraph::new(details)
            .style(theme.text_dim())
            .alignment(Alignment::Center);
        frame.render_widget(detail_widget, detail_area);
    }
}

/// Packages tab: List packages for selected generation
fn render_packages_tab(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    // FIX: Add background style first
    let block = Block::default()
        .style(theme.block_style())
        .title(format!(
            " Packages · Generation #{} ",
            app.packages_gen_id.unwrap_or(0)
        ))
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Filter input
    let filter_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 1,
    };
    let filter_text = format!("Filter: {}_", app.packages_filter);
    let filter_widget = Paragraph::new(filter_text).style(theme.text());
    frame.render_widget(filter_widget, filter_area);

    // Package list
    let list_area = Rect {
        x: inner.x,
        y: inner.y + 2,
        width: inner.width,
        height: inner.height.saturating_sub(3),
    };

    let filtered: Vec<_> = app.packages_list
        .iter()
        .filter(|p| {
            app.packages_filter.is_empty() 
            || p.name.to_lowercase().contains(&app.packages_filter.to_lowercase())
        })
        .collect();

    if filtered.is_empty() {
        let empty_msg = Paragraph::new("No packages match filter")
            .style(theme.text_dim())
            .alignment(Alignment::Center);
        frame.render_widget(empty_msg, list_area);
        return;
    }

    // Table header
    let header = Row::new(vec![
        Cell::from("NAME").style(theme.title()),
        Cell::from("VERSION").style(theme.title()),
        Cell::from("SIZE").style(theme.title()),
    ]);

    // Table rows
    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(i, pkg)| {
            let style = if i == app.packages_selected {
                theme.selected()
            } else {
                theme.text()
            };

            Row::new(vec![
                Cell::from(pkg.name.clone()),
                Cell::from(pkg.version.clone()),
                Cell::from(pkg.formatted_size()),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(50),
            Constraint::Percentage(30),
            Constraint::Percentage(20),
        ],
    )
    .header(header);

    frame.render_widget(table, list_area);

    // Show count at bottom
    let count_area = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(1),
        width: inner.width,
        height: 1,
    };
    let count_text = format!(
        "{} / {} packages",
        app.packages_selected.saturating_add(1).min(filtered.len()),
        filtered.len()
    );
    let count_widget = Paragraph::new(count_text)
        .style(theme.text_dim())
        .alignment(Alignment::Right);
    frame.render_widget(count_widget, count_area);
}

/// Diff tab: Compare two generations
fn render_diff_tab(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    // FIX: Add background style first
    let block = Block::default()
        .style(theme.block_style())
        .title(" Compare Generations ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Dropdown selectors at top
    let selector_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 2,
    };

    let from_label = format!(
        "From: [#{} ▼]",
        app.diff_from_gen.map(|g| g.to_string()).unwrap_or_else(|| "Select".into())
    );
    let to_label = format!(
        "To: [#{} ▼]",
        app.diff_to_gen.map(|g| g.to_string()).unwrap_or_else(|| "Select".into())
    );

    let from_style = if app.diff_focus == 0 {
        theme.selected()
    } else {
        theme.text()
    };
    let to_style = if app.diff_focus == 1 {
        theme.selected()
    } else {
        theme.text()
    };

    let selector_line = Line::from(vec![
        Span::styled(from_label, from_style),
        Span::raw("              "),
        Span::styled(to_label, to_style),
    ]);

    let selector_widget = Paragraph::new(selector_line);
    frame.render_widget(selector_widget, selector_area);

    // Diff results
    let diff_area = Rect {
        x: inner.x,
        y: inner.y + 3,
        width: inner.width,
        height: inner.height.saturating_sub(4),
    };

    if let Some(diff) = &app.current_diff {
        render_diff_content(frame, diff, app.diff_scroll, theme, diff_area);
    } else {
        let hint = Paragraph::new("Select two generations to compare")
            .style(theme.text_dim())
            .alignment(Alignment::Center);
        frame.render_widget(hint, diff_area);
    }
}

/// Render diff content
fn render_diff_content(
    frame: &mut Frame,
    diff: &GenerationDiff,
    scroll: usize,
    theme: &Theme,
    area: Rect,
) {
    let mut lines: Vec<Line> = Vec::new();

    // Summary
    lines.push(Line::styled(diff.summary(), theme.title()));
    lines.push(Line::raw(""));

    // Added
    if !diff.added.is_empty() {
        lines.push(Line::styled(
            format!("Added ({})", diff.added.len()),
            theme.diff_added(),
        ));
        for pkg in &diff.added {
            lines.push(Line::from(vec![
                Span::styled(" + ", theme.diff_added()),
                Span::styled(&pkg.name, theme.text()),
                Span::raw(" "),
                Span::styled(&pkg.version, theme.text_dim()),
            ]));
        }
        lines.push(Line::raw(""));
    }

    // Removed
    if !diff.removed.is_empty() {
        lines.push(Line::styled(
            format!("Removed ({})", diff.removed.len()),
            theme.diff_removed(),
        ));
        for pkg in &diff.removed {
            lines.push(Line::from(vec![
                Span::styled(" - ", theme.diff_removed()),
                Span::styled(&pkg.name, theme.text()),
                Span::raw(" "),
                Span::styled(&pkg.version, theme.text_dim()),
            ]));
        }
        lines.push(Line::raw(""));
    }

    // Updated
    if !diff.updated.is_empty() {
        lines.push(Line::styled(
            format!("Updated ({})", diff.updated.len()),
            theme.diff_updated(),
        ));
        for upd in &diff.updated {
            let mut spans = vec![
                Span::styled(" ~ ", theme.diff_updated()),
                Span::styled(&upd.name, theme.text()),
                Span::raw(" "),
                Span::styled(&upd.old_version, theme.text_dim()),
                Span::raw(" → "),
                Span::styled(&upd.new_version, theme.text()),
            ];
            if upd.is_kernel {
                spans.push(Span::styled(" ⚠ Kernel", theme.warning()));
            } else if upd.is_security {
                spans.push(Span::styled(" ⚠ Security", theme.warning()));
            }
            lines.push(Line::from(spans));
        }
    }

    // Apply scroll
    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(scroll)
        .take(area.height as usize)
        .collect();

    let content = Paragraph::new(visible_lines).wrap(Wrap { trim: false });
    frame.render_widget(content, area);
}

/// Manage tab: Restore, delete, pin generations
fn render_manage_tab(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    // FIX: Add background style first
    let block = Block::default()
        .style(theme.block_style())
        .title(" Manage Generations ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Profile selector
    let profile_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 1,
    };
    let profile_label = format!(
        "Profile: [{}]  (Tab to switch)",
        if app.manage_profile == ProfileType::System { "System" } else { "Home-Manager" }
    );
    let profile_widget = Paragraph::new(profile_label).style(theme.text());
    frame.render_widget(profile_widget, profile_area);

    // Generation table
    let table_area = Rect {
        x: inner.x,
        y: inner.y + 2,
        width: inner.width,
        height: inner.height.saturating_sub(6),
    };

    let generations = if app.manage_profile == ProfileType::System {
        &app.system_generations
    } else {
        app.home_manager_generations.as_ref().unwrap_or(&app.system_generations)
    };

    // Header
    let header = Row::new(vec![
        Cell::from("").style(theme.title()),
        Cell::from("GEN").style(theme.title()),
        Cell::from("DATE").style(theme.title()),
        Cell::from("SIZE").style(theme.title()),
        Cell::from("STATUS").style(theme.title()),
    ]);

    // Rows
    let rows: Vec<Row> = generations
        .iter()
        .enumerate()
        .map(|(i, gen)| {
            let selected_marker = if app.manage_selected.contains(&gen.id) {
                "■"
            } else {
                "□"
            };

            let status = if gen.is_current {
                "● current"
            } else if gen.is_pinned {
                "★ pinned"
            } else if gen.in_bootloader {
                "⚡ boot"
            } else {
                ""
            };

            let style = if i == app.manage_cursor {
                theme.selected()
            } else {
                theme.text()
            };

            Row::new(vec![
                Cell::from(selected_marker),
                Cell::from(format!("#{}", gen.id)),
                Cell::from(gen.formatted_date()),
                Cell::from(gen.formatted_size()),
                Cell::from(status),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Length(8),
            Constraint::Length(16),
            Constraint::Length(12),
            Constraint::Min(10),
        ],
    )
    .header(header);

    frame.render_widget(table, table_area);

    // Actions help at bottom
    let actions_area = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(3),
        width: inner.width,
        height: 2,
    };

    let selected_count = app.manage_selected.len();
    let actions_text = if selected_count > 0 {
        format!(
            "{} selected · [R] Restore  [D] Delete  [P] Pin/Unpin  [C] Clear",
            selected_count
        )
    } else {
        "[Space] Select  [A] Select All  [R] Restore  [P] Pin/Unpin".to_string()
    };

    let actions_widget = Paragraph::new(actions_text)
        .style(theme.text_dim())
        .alignment(Alignment::Center);
    frame.render_widget(actions_widget, actions_area);
}

/// Settings tab
fn render_settings_tab(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    // FIX: Add background style first
    let block = Block::default()
        .style(theme.block_style())
        .title(" Settings ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let settings = [
        ("Theme", app.config.theme.as_str()),
        ("Layout", app.config.layout.as_str()),
        ("Show NixOS Version", bool_str(app.config.display.show_nixos_version)),
        ("Show Kernel Version", bool_str(app.config.display.show_kernel_version)),
        ("Show Package Count", bool_str(app.config.display.show_package_count)),
        ("Show Size", bool_str(app.config.display.show_size)),
        ("Show Boot Entry", bool_str(app.config.display.show_boot_entry)),
    ];

    let items: Vec<ListItem> = settings
        .iter()
        .enumerate()
        .map(|(i, (label, value))| {
            let style = if i == app.settings_selected {
                theme.selected()
            } else {
                theme.text()
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<24}", label), style),
                Span::styled(format!("[{}]", value), Style::default().fg(theme.accent)),
            ]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);

    // Config path at bottom
    let config_path = crate::config::Config::path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "Unknown".into());

    let path_area = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(2),
        width: inner.width,
        height: 1,
    };
    let path_widget = Paragraph::new(format!("Config: {}", config_path))
        .style(theme.text_dim());
    frame.render_widget(path_widget, path_area);
}

fn bool_str(b: bool) -> &'static str {
    if b { "✓" } else { " " }
}