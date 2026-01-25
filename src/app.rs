//! Application state and event handling
//!
//! This is the core of nixhist, managing:
//! - Application state across all tabs
//! - Event handling (keyboard input)
//! - State transitions and data loading

use crate::config::{Config, LayoutMode};
use crate::nix::{
    self, CommandResult, GenerationSource, SystemInfo,
    delete_generations, get_packages, list_generations, restore_generation,
};
use crate::types::{Generation, GenerationDiff, Package, ProfileType, Tab};
use crate::ui::Theme;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use std::collections::HashSet;
use std::time::Instant;

/// Main application state
pub struct App {
    // Core state
    pub should_quit: bool,
    pub active_tab: Tab,
    pub config: Config,
    pub theme: Theme,
    pub system_info: SystemInfo,
    pub dry_run: bool,

    // System generations
    pub system_generations: Vec<Generation>,
    pub system_source: GenerationSource,

    // Home-Manager generations (optional)
    pub home_manager_generations: Option<Vec<Generation>>,
    pub home_manager_source: Option<GenerationSource>,

    // Overview tab state
    pub overview_focus: usize,           // 0 = System, 1 = HM
    pub overview_system_selected: usize,
    pub overview_hm_selected: usize,

    // Packages tab state
    pub packages_list: Vec<Package>,
    pub packages_gen_id: Option<u32>,
    pub packages_profile: ProfileType,
    pub packages_selected: usize,
    pub packages_filter: String,

    // Diff tab state - FIX: Add cursors for selection lists
    pub diff_focus: usize,               // 0 = From list, 1 = To list
    pub diff_from_cursor: usize,         // NEW: Cursor in From list
    pub diff_to_cursor: usize,           // NEW: Cursor in To list
    pub diff_from_gen: Option<u32>,
    pub diff_to_gen: Option<u32>,
    pub diff_scroll: usize,
    pub current_diff: Option<GenerationDiff>,

    // Manage tab state
    pub manage_profile: ProfileType,
    pub manage_cursor: usize,
    pub manage_selected: HashSet<u32>,

    // Settings tab state
    pub settings_selected: usize,

    // Popup state
    pub popup: PopupState,

    // Flash message (temporary feedback)
    pub flash_message: Option<(String, bool, Instant)>, // (message, is_error, timestamp)

    // Undo state
    pub pending_undo: Option<PendingUndo>,
}

/// Popup overlay state
#[derive(Debug, Clone)]
pub enum PopupState {
    None,
    Confirm {
        title: String,
        message: String,
        command: String,
    },
    Error {
        title: String,
        message: String,
    },
    Undo {
        message: String,
        seconds_remaining: u8,
    },
    Loading {
        message: String,
    },
}

/// Pending undo action
#[derive(Debug, Clone)]
pub struct PendingUndo {
    pub action: UndoAction,
    pub started_at: Instant,
}

/// Action that can be undone
#[derive(Debug, Clone)]
pub enum UndoAction {
    Delete {
        profile: ProfileType,
        generation_ids: Vec<u32>,
    },
}

/// Application state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Normal,
    FilterInput,
    DropdownOpen,
    ConfirmAction,
    ShowError,
    UndoCountdown,
    Loading,
}

impl App {
    /// Create a new App instance
    pub fn new(system_info: SystemInfo, config: Config, dry_run: bool) -> Result<Self> {
        let theme = Theme::from_name(config.theme);

        // System generations source
        let system_source = GenerationSource {
            profile_type: ProfileType::System,
            profile_path: system_info.system_profile.clone(),
        };

        // Load system generations
        let mut system_generations = list_generations(&system_source)?;
        
        // Apply pinned status from config
        for gen in &mut system_generations {
            gen.is_pinned = config.is_system_pinned(gen.id);
        }

        // Home-Manager source (if detected)
        let (home_manager_source, home_manager_generations) = 
            if let Some(hm_info) = &system_info.home_manager {
                let source = GenerationSource {
                    profile_type: ProfileType::HomeManager,
                    profile_path: hm_info.profile_path.clone(),
                };
                
                match list_generations(&source) {
                    Ok(mut gens) => {
                        for gen in &mut gens {
                            gen.is_pinned = config.is_home_manager_pinned(gen.id);
                        }
                        (Some(source), Some(gens))
                    }
                    Err(_) => (None, None), // Graceful degradation
                }
            } else {
                (None, None)
            };

        Ok(Self {
            should_quit: false,
            active_tab: Tab::Overview,
            config,
            theme,
            system_info,
            dry_run,

            system_generations,
            system_source,

            home_manager_generations,
            home_manager_source,

            overview_focus: 0,
            overview_system_selected: 0,
            overview_hm_selected: 0,

            packages_list: Vec::new(),
            packages_gen_id: None,
            packages_profile: ProfileType::System,
            packages_selected: 0,
            packages_filter: String::new(),

            diff_focus: 0,
            diff_from_cursor: 0,      // NEW: Initialize cursors
            diff_to_cursor: 0,        // NEW: Initialize cursors
            diff_from_gen: None,
            diff_to_gen: None,
            diff_scroll: 0,
            current_diff: None,

            manage_profile: ProfileType::System,
            manage_cursor: 0,
            manage_selected: HashSet::new(),

            settings_selected: 0,

            popup: PopupState::None,
            flash_message: None,
            pending_undo: None,
        })
    }

    /// Get current app state
    pub fn state(&self) -> AppState {
        match &self.popup {
            PopupState::None => {
                if self.active_tab == Tab::Packages && !self.packages_filter.is_empty() {
                    AppState::FilterInput
                } else {
                    AppState::Normal
                }
            }
            PopupState::Confirm { .. } => AppState::ConfirmAction,
            PopupState::Error { .. } => AppState::ShowError,
            PopupState::Undo { .. } => AppState::UndoCountdown,
            PopupState::Loading { .. } => AppState::Loading,
        }
    }

    /// Check if side-by-side layout should be used
    pub fn should_use_side_by_side(&self, terminal_width: u16) -> bool {
        match self.config.layout {
            LayoutMode::SideBySide => true,
            LayoutMode::TabsOnly => false,
            LayoutMode::Auto => terminal_width >= 100,
        }
    }

    /// Handle a key event
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Clear expired flash messages
        if let Some((_, _, instant)) = &self.flash_message {
            if instant.elapsed().as_secs() >= 3 {
                self.flash_message = None;
            }
        }

        // Handle based on current state
        match self.state() {
            AppState::ConfirmAction => self.handle_confirm_key(key),
            AppState::ShowError => self.handle_error_key(key),
            AppState::UndoCountdown => self.handle_undo_key(key),
            AppState::Loading => Ok(()), // Ignore input while loading
            AppState::Normal | AppState::FilterInput | AppState::DropdownOpen => {
                self.handle_normal_key(key)
            }
        }
    }

    /// Handle key in normal state
    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        // Global keys (work in all tabs)
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                return Ok(());
            }
            KeyCode::Char('1') => self.active_tab = Tab::Overview,
            KeyCode::Char('2') => self.active_tab = Tab::Packages,
            KeyCode::Char('3') => self.active_tab = Tab::Diff,
            KeyCode::Char('4') => self.active_tab = Tab::Manage,
            KeyCode::Char('5') => self.active_tab = Tab::Settings,
            _ => {}
        }

        // Tab-specific handling
        match self.active_tab {
            Tab::Overview => self.handle_overview_key(key),
            Tab::Packages => self.handle_packages_key(key),
            Tab::Diff => self.handle_diff_key(key),
            Tab::Manage => self.handle_manage_key(key),
            Tab::Settings => self.handle_settings_key(key),
        }
    }

    /// Handle keys in Overview tab
    fn handle_overview_key(&mut self, key: KeyEvent) -> Result<()> {
        let has_hm = self.home_manager_generations.is_some();

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.overview_focus == 0 {
                    if self.overview_system_selected < self.system_generations.len().saturating_sub(1) {
                        self.overview_system_selected += 1;
                    }
                } else if let Some(hm) = &self.home_manager_generations {
                    if self.overview_hm_selected < hm.len().saturating_sub(1) {
                        self.overview_hm_selected += 1;
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.overview_focus == 0 {
                    self.overview_system_selected = self.overview_system_selected.saturating_sub(1);
                } else {
                    self.overview_hm_selected = self.overview_hm_selected.saturating_sub(1);
                }
            }
            KeyCode::Char('g') => {
                if self.overview_focus == 0 {
                    self.overview_system_selected = 0;
                } else {
                    self.overview_hm_selected = 0;
                }
            }
            KeyCode::Char('G') => {
                if self.overview_focus == 0 {
                    self.overview_system_selected = self.system_generations.len().saturating_sub(1);
                } else if let Some(hm) = &self.home_manager_generations {
                    self.overview_hm_selected = hm.len().saturating_sub(1);
                }
            }
            KeyCode::Tab => {
                if has_hm {
                    self.overview_focus = (self.overview_focus + 1) % 2;
                }
            }
            KeyCode::Enter => {
                // Switch to Packages tab with selected generation
                let (gen, profile) = if self.overview_focus == 0 {
                    (self.system_generations.get(self.overview_system_selected), ProfileType::System)
                } else {
                    let hm = self.home_manager_generations.as_ref();
                    (hm.and_then(|g| g.get(self.overview_hm_selected)), ProfileType::HomeManager)
                };

                if let Some(gen) = gen {
                    self.load_packages(gen.id, profile)?;
                    self.active_tab = Tab::Packages;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keys in Packages tab
    fn handle_packages_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('/') => {
                // Start filter input
                self.packages_filter.clear();
            }
            KeyCode::Char(c) if !self.packages_filter.is_empty() || key.code == KeyCode::Char('/') => {
                if c != '/' {
                    self.packages_filter.push(c);
                }
                self.packages_selected = 0;
            }
            KeyCode::Backspace => {
                self.packages_filter.pop();
                self.packages_selected = 0;
            }
            KeyCode::Esc => {
                self.packages_filter.clear();
                self.packages_selected = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let filtered_count = self.filtered_packages_count();
                if self.packages_selected < filtered_count.saturating_sub(1) {
                    self.packages_selected += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.packages_selected = self.packages_selected.saturating_sub(1);
            }
            KeyCode::Char('g') => {
                self.packages_selected = 0;
            }
            KeyCode::Char('G') => {
                self.packages_selected = self.filtered_packages_count().saturating_sub(1);
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keys in Diff tab - COMPLETELY REWRITTEN
    fn handle_diff_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Tab => {
                // Switch between From and To lists
                self.diff_focus = (self.diff_focus + 1) % 2;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                // Navigate in active list
                if self.diff_focus == 0 {
                    if self.diff_from_cursor < self.system_generations.len().saturating_sub(1) {
                        self.diff_from_cursor += 1;
                    }
                } else {
                    if self.diff_to_cursor < self.system_generations.len().saturating_sub(1) {
                        self.diff_to_cursor += 1;
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                // Navigate in active list
                if self.diff_focus == 0 {
                    self.diff_from_cursor = self.diff_from_cursor.saturating_sub(1);
                } else {
                    self.diff_to_cursor = self.diff_to_cursor.saturating_sub(1);
                }
            }
            KeyCode::Char('g') => {
                // Jump to top of active list
                if self.diff_focus == 0 {
                    self.diff_from_cursor = 0;
                } else {
                    self.diff_to_cursor = 0;
                }
            }
            KeyCode::Char('G') => {
                // Jump to bottom of active list
                let max = self.system_generations.len().saturating_sub(1);
                if self.diff_focus == 0 {
                    self.diff_from_cursor = max;
                } else {
                    self.diff_to_cursor = max;
                }
            }
            KeyCode::Enter => {
                // Select generation from active list
                let gen_id = if self.diff_focus == 0 {
                    self.system_generations.get(self.diff_from_cursor).map(|g| g.id)
                } else {
                    self.system_generations.get(self.diff_to_cursor).map(|g| g.id)
                };

                if let Some(id) = gen_id {
                    if self.diff_focus == 0 {
                        self.diff_from_gen = Some(id);
                    } else {
                        self.diff_to_gen = Some(id);
                    }

                    // Automatically calculate diff if both are selected
                    if self.diff_from_gen.is_some() && self.diff_to_gen.is_some() {
                        self.calculate_diff()?;
                    }
                }
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                // Clear selection
                self.diff_from_gen = None;
                self.diff_to_gen = None;
                self.current_diff = None;
                self.diff_scroll = 0;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keys in Manage tab
    fn handle_manage_key(&mut self, key: KeyEvent) -> Result<()> {
        let generations = if self.manage_profile == ProfileType::System {
            &self.system_generations
        } else {
            self.home_manager_generations.as_ref().unwrap_or(&self.system_generations)
        };

        match key.code {
            KeyCode::Tab => {
                if self.home_manager_generations.is_some() {
                    self.manage_profile = match self.manage_profile {
                        ProfileType::System => ProfileType::HomeManager,
                        ProfileType::HomeManager => ProfileType::System,
                    };
                    self.manage_cursor = 0;
                    self.manage_selected.clear();
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.manage_cursor < generations.len().saturating_sub(1) {
                    self.manage_cursor += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.manage_cursor = self.manage_cursor.saturating_sub(1);
            }
            KeyCode::Char(' ') => {
                // Toggle selection
                if let Some(gen) = generations.get(self.manage_cursor) {
                    if !gen.is_current { // Can't select current generation
                        if self.manage_selected.contains(&gen.id) {
                            self.manage_selected.remove(&gen.id);
                        } else {
                            self.manage_selected.insert(gen.id);
                        }
                    }
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                // Select all (except current and pinned)
                for gen in generations {
                    if !gen.is_current && !gen.is_pinned {
                        self.manage_selected.insert(gen.id);
                    }
                }
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                // Clear selection
                self.manage_selected.clear();
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                // Pin/unpin
                if let Some(gen) = generations.get(self.manage_cursor) {
                    self.toggle_pin(gen.id)?;
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // Restore
                self.prompt_restore()?;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                // Delete
                self.prompt_delete()?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keys in Settings tab
    fn handle_settings_key(&mut self, key: KeyEvent) -> Result<()> {
        let settings_count = 7; // Number of settings items

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.settings_selected < settings_count - 1 {
                    self.settings_selected += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.settings_selected = self.settings_selected.saturating_sub(1);
            }
            KeyCode::Enter => {
                // Toggle/cycle setting
                match self.settings_selected {
                    0 => { // Theme
                        self.config.theme = self.config.theme.next();
                        self.theme = Theme::from_name(self.config.theme);
                    }
                    1 => { // Layout
                        self.config.layout = self.config.layout.next();
                    }
                    2 => self.config.display.show_nixos_version = !self.config.display.show_nixos_version,
                    3 => self.config.display.show_kernel_version = !self.config.display.show_kernel_version,
                    4 => self.config.display.show_package_count = !self.config.display.show_package_count,
                    5 => self.config.display.show_size = !self.config.display.show_size,
                    6 => self.config.display.show_boot_entry = !self.config.display.show_boot_entry,
                    _ => {}
                }
                // Save config
                if let Err(e) = self.config.save() {
                    self.show_error("Save Failed", &e.to_string());
                } else {
                    self.show_flash("Settings saved", false);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keys in confirm popup
    fn handle_confirm_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.execute_pending_action()?;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.popup = PopupState::None;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keys in error popup
    fn handle_error_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('o') | KeyCode::Enter | KeyCode::Esc => {
                self.popup = PopupState::None;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keys in undo countdown
    fn handle_undo_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('u') | KeyCode::Char('U') => {
                // Perform undo
                self.perform_undo()?;
            }
            KeyCode::Esc => {
                // Confirm deletion (stop countdown)
                self.pending_undo = None;
                self.popup = PopupState::None;
            }
            _ => {}
        }
        Ok(())
    }

    /// Update undo countdown timer
    pub fn update_undo_timer(&mut self) -> Result<()> {
        if let Some(pending) = &self.pending_undo {
            let elapsed = pending.started_at.elapsed().as_secs() as u8;
            let remaining = 10u8.saturating_sub(elapsed);

            if remaining == 0 {
                // Time's up - action is confirmed
                self.pending_undo = None;
                self.popup = PopupState::None;
                self.show_flash("Action confirmed", false);
            } else {
                // Update countdown display
                if let PopupState::Undo { message, .. } = &self.popup {
                    self.popup = PopupState::Undo {
                        message: message.clone(),
                        seconds_remaining: remaining,
                    };
                }
            }
        }
        Ok(())
    }

    // === HELPER METHODS ===

    /// Load packages for a generation
    fn load_packages(&mut self, gen_id: u32, profile: ProfileType) -> Result<()> {
        let source = if profile == ProfileType::System {
            &self.system_source
        } else {
            self.home_manager_source.as_ref().unwrap_or(&self.system_source)
        };

        let gen_path = source.profile_path.parent()
            .unwrap_or(&source.profile_path)
            .join(format!(
                "{}-{}-link",
                if profile == ProfileType::System { "system" } else { "home-manager" },
                gen_id
            ));

        self.packages_list = get_packages(&gen_path).unwrap_or_default();
        self.packages_gen_id = Some(gen_id);
        self.packages_profile = profile;
        self.packages_selected = 0;
        self.packages_filter.clear();

        Ok(())
    }

    /// Count filtered packages
    fn filtered_packages_count(&self) -> usize {
        if self.packages_filter.is_empty() {
            self.packages_list.len()
        } else {
            self.packages_list
                .iter()
                .filter(|p| p.name.to_lowercase().contains(&self.packages_filter.to_lowercase()))
                .count()
        }
    }

    /// Calculate diff between two generations
    fn calculate_diff(&mut self) -> Result<()> {
        let (from_id, to_id) = match (self.diff_from_gen, self.diff_to_gen) {
            (Some(from), Some(to)) => (from, to),
            _ => return Ok(()),
        };

        let source = &self.system_source;
        let parent = source.profile_path.parent().unwrap_or(&source.profile_path);

        let from_path = parent.join(format!("system-{}-link", from_id));
        let to_path = parent.join(format!("system-{}-link", to_id));

        let from_packages = get_packages(&from_path).unwrap_or_default();
        let to_packages = get_packages(&to_path).unwrap_or_default();

        self.current_diff = Some(GenerationDiff::calculate(&from_packages, &to_packages));
        self.diff_scroll = 0;

        Ok(())
    }

    /// Toggle pin status for a generation
    fn toggle_pin(&mut self, gen_id: u32) -> Result<()> {
        match self.manage_profile {
            ProfileType::System => {
                self.config.toggle_system_pin(gen_id);
                if let Some(gen) = self.system_generations.iter_mut().find(|g| g.id == gen_id) {
                    gen.is_pinned = self.config.is_system_pinned(gen_id);
                }
            }
            ProfileType::HomeManager => {
                self.config.toggle_home_manager_pin(gen_id);
                if let Some(gens) = &mut self.home_manager_generations {
                    if let Some(gen) = gens.iter_mut().find(|g| g.id == gen_id) {
                        gen.is_pinned = self.config.is_home_manager_pinned(gen_id);
                    }
                }
            }
        }
        self.config.save()?;
        self.show_flash("Pin status updated", false);
        Ok(())
    }

    /// Prompt for restore confirmation
    fn prompt_restore(&mut self) -> Result<()> {
        let generations = if self.manage_profile == ProfileType::System {
            &self.system_generations
        } else {
            self.home_manager_generations.as_ref().unwrap_or(&self.system_generations)
        };

        let gen = match generations.get(self.manage_cursor) {
            Some(g) if !g.is_current => g,
            _ => {
                self.show_flash("Cannot restore current generation", true);
                return Ok(());
            }
        };

        let source = if self.manage_profile == ProfileType::System {
            &self.system_source
        } else {
            self.home_manager_source.as_ref().unwrap_or(&self.system_source)
        };

        let command = nix::commands::get_restore_command_preview(
            &source.profile_path,
            gen.id,
            self.manage_profile,
        );

        self.popup = PopupState::Confirm {
            title: "Confirm Restore".into(),
            message: format!(
                "Restore {} generation #{}?\n\nDate: {}\nVersion: {}",
                self.manage_profile.as_str(),
                gen.id,
                gen.formatted_date(),
                gen.nixos_version.as_deref().unwrap_or("Unknown"),
            ),
            command,
        };

        Ok(())
    }

    /// Prompt for delete confirmation
    fn prompt_delete(&mut self) -> Result<()> {
        let ids: Vec<u32> = if self.manage_selected.is_empty() {
            // Delete single (under cursor)
            let generations = if self.manage_profile == ProfileType::System {
                &self.system_generations
            } else {
                self.home_manager_generations.as_ref().unwrap_or(&self.system_generations)
            };

            match generations.get(self.manage_cursor) {
                Some(g) if !g.is_current && !g.is_pinned => vec![g.id],
                Some(g) if g.is_current => {
                    self.show_flash("Cannot delete current generation", true);
                    return Ok(());
                }
                Some(g) if g.is_pinned => {
                    self.show_flash("Cannot delete pinned generation (unpin first)", true);
                    return Ok(());
                }
                _ => return Ok(()),
            }
        } else {
            // Delete selected
            self.manage_selected.iter().copied().collect()
        };

        if ids.is_empty() {
            return Ok(());
        }

        let source = if self.manage_profile == ProfileType::System {
            &self.system_source
        } else {
            self.home_manager_source.as_ref().unwrap_or(&self.system_source)
        };

        let command = nix::commands::get_delete_command_preview(
            &source.profile_path,
            &ids,
            self.manage_profile,
        );

        self.popup = PopupState::Confirm {
            title: "Confirm Delete".into(),
            message: format!(
                "Delete {} generation(s)?\n\nIDs: {:?}\n\n⚠ This cannot be undone!",
                ids.len(),
                ids,
            ),
            command,
        };

        Ok(())
    }

    /// Execute the pending confirmed action
    fn execute_pending_action(&mut self) -> Result<()> {
        // Get action details from popup
        let (title, _message, command) = match &self.popup {
            PopupState::Confirm { title, message, command } => {
                (title.clone(), message.clone(), command.clone())
            }
            _ => return Ok(()),
        };

        self.popup = PopupState::Loading {
            message: "Executing...".into(),
        };

        let result = if title.contains("Restore") {
            self.execute_restore()
        } else if title.contains("Delete") {
            self.execute_delete()
        } else {
            Ok(CommandResult {
                success: false,
                message: "Unknown action".into(),
                command,
            })
        };

        match result {
            Ok(cmd_result) if cmd_result.success => {
                self.popup = PopupState::None;
                self.show_flash(&cmd_result.message, false);
                self.refresh_generations()?;
            }
            Ok(cmd_result) => {
                self.show_error("Command Failed", &cmd_result.message);
            }
            Err(e) => {
                self.show_error("Error", &e.to_string());
            }
        }

        Ok(())
    }

    /// Execute restore action
    fn execute_restore(&mut self) -> Result<CommandResult> {
        let generations = if self.manage_profile == ProfileType::System {
            &self.system_generations
        } else {
            self.home_manager_generations.as_ref().unwrap_or(&self.system_generations)
        };

        let gen = generations.get(self.manage_cursor)
            .ok_or_else(|| anyhow::anyhow!("No generation selected"))?;

        let source = if self.manage_profile == ProfileType::System {
            &self.system_source
        } else {
            self.home_manager_source.as_ref().unwrap_or(&self.system_source)
        };

        restore_generation(
            &source.profile_path,
            gen.id,
            self.manage_profile,
            self.dry_run,
        )
    }

    /// Execute delete action
    fn execute_delete(&mut self) -> Result<CommandResult> {
        let ids: Vec<u32> = if self.manage_selected.is_empty() {
            let generations = if self.manage_profile == ProfileType::System {
                &self.system_generations
            } else {
                self.home_manager_generations.as_ref().unwrap_or(&self.system_generations)
            };

            generations.get(self.manage_cursor)
                .map(|g| vec![g.id])
                .unwrap_or_default()
        } else {
            self.manage_selected.iter().copied().collect()
        };

        let source = if self.manage_profile == ProfileType::System {
            &self.system_source
        } else {
            self.home_manager_source.as_ref().unwrap_or(&self.system_source)
        };

        let result = delete_generations(
            &source.profile_path,
            &ids,
            self.manage_profile,
            self.dry_run,
        )?;

        if result.success && !self.dry_run {
            // Start undo countdown
            self.pending_undo = Some(PendingUndo {
                action: UndoAction::Delete {
                    profile: self.manage_profile,
                    generation_ids: ids.clone(),
                },
                started_at: Instant::now(),
            });

            self.popup = PopupState::Undo {
                message: format!("Deleted {} generation(s)", ids.len()),
                seconds_remaining: 10,
            };
        }

        self.manage_selected.clear();

        Ok(result)
    }

    /// Perform undo action
    fn perform_undo(&mut self) -> Result<()> {
        // For delete, we can't actually undo - just notify user
        self.pending_undo = None;
        self.popup = PopupState::None;
        self.show_flash("Cannot undo delete - generation is gone", true);
        Ok(())
    }

    /// Refresh generations from disk
    fn refresh_generations(&mut self) -> Result<()> {
        self.system_generations = list_generations(&self.system_source)?;
        for gen in &mut self.system_generations {
            gen.is_pinned = self.config.is_system_pinned(gen.id);
        }

        if let Some(source) = &self.home_manager_source {
            if let Ok(mut gens) = list_generations(source) {
                for gen in &mut gens {
                    gen.is_pinned = self.config.is_home_manager_pinned(gen.id);
                }
                self.home_manager_generations = Some(gens);
            }
        }

        Ok(())
    }

    /// Show an error popup
    fn show_error(&mut self, title: &str, message: &str) {
        self.popup = PopupState::Error {
            title: title.into(),
            message: message.into(),
        };
    }

    /// Show a flash message
    fn show_flash(&mut self, message: &str, is_error: bool) {
        self.flash_message = Some((message.into(), is_error, Instant::now()));
    }
}