//! nixhist - NixOS Generation Dashboard
//!
//! A TUI for viewing, comparing, and managing NixOS generations.
//!
//! Features:
//! - View System and Home-Manager generations
//! - Compare packages between generations
//! - Restore to previous generations
//! - Delete old generations (with undo countdown)
//! - Pin important generations
//!
//! Usage: nixhist [--dry-run]

mod app;
mod config;
mod nix;
mod types;
mod ui;

use anyhow::{Context, Result};
use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::stdout;
use std::time::Duration;

fn main() -> Result<()> {
    // Parse arguments
    let args: Vec<String> = std::env::args().collect();
    let dry_run = args.iter().any(|a| a == "--dry-run" || a == "-n");

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }

    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("nixhist {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Run the application
    let result = run_app(dry_run);

    // Always try to restore terminal state, even on error
    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }

    Ok(())
}

fn print_help() {
    println!(
        r#"nixhist - NixOS Generation Dashboard

USAGE:
    nixhist [OPTIONS]

OPTIONS:
    -n, --dry-run    Show what would be done without executing
    -h, --help       Print help information
    -v, --version    Print version information

KEYBINDINGS:
    1-5              Switch tabs
    j/k              Navigate up/down
    Tab              Switch panel/focus
    Enter            Select/confirm
    Space            Toggle selection (Manage tab)
    R                Restore generation
    D                Delete generation(s)
    P                Pin/unpin generation
    /                Filter (Packages tab)
    q                Quit

TABS:
    [1] Overview     View all generations
    [2] Packages     Browse packages in a generation
    [3] Diff         Compare two generations
    [4] Manage       Restore, delete, pin generations
    [5] Settings     Configure theme and display options

CONFIG:
    ~/.config/nixhist/config.toml
"#
    );
}

fn run_app(dry_run: bool) -> Result<()> {
    // Detect system configuration
    eprintln!("Detecting system configuration...");
    let system_info = nix::detect_system()
        .context("Failed to detect system configuration")?;

    eprintln!("Hostname: {}", system_info.hostname);
    eprintln!("Uses flakes: {}", system_info.uses_flakes);
    eprintln!(
        "Home-Manager: {}",
        if system_info.home_manager.is_some() {
            "detected"
        } else {
            "not found"
        }
    );

    // Load configuration
    let config = config::Config::load()
        .context("Failed to load configuration")?;

    // Create application state
    eprintln!("Loading generations...");
    let mut app = App::new(system_info, config, dry_run)
        .context("Failed to initialize application")?;

    if dry_run {
        eprintln!("Running in dry-run mode (no changes will be made)");
    }

    // Setup terminal
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("Failed to setup terminal")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)
        .context("Failed to create terminal")?;

    // Run main loop
    let result = main_loop(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .context("Failed to restore terminal")?;
    terminal.show_cursor().context("Failed to show cursor")?;

    result
}

fn main_loop<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        // Render UI
        terminal.draw(|frame| {
            ui::render(frame, app);
        })?;

        // Update undo timer if active
        app.update_undo_timer()?;

        // Poll for events with timeout (for timer updates)
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Only handle key press events (not release)
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key)?;
                }
            }
        }

        // Check if should quit
        if app.should_quit {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_does_not_panic() {
        print_help();
    }
}
