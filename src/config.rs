//! Configuration management for nixhist
//!
//! Handles loading, saving, and default configuration values.
//! Config file location: ~/.config/nixhist/config.toml

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: ThemeName,
    pub layout: LayoutMode,
    pub display: DisplayOptions,
    pub pinned: PinnedGenerations,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeName::Gruvbox,
            layout: LayoutMode::Auto,
            display: DisplayOptions::default(),
            pinned: PinnedGenerations::default(),
        }
    }
}

impl Config {
    /// Get the config file path
    pub fn path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("nixhist");
        Ok(config_dir.join("config.toml"))
    }

    /// Load config from file, or create default if not exists
    pub fn load() -> Result<Self> {
        let path = Self::path()?;

        if !path.exists() {
            let config = Config::default();
            config.save()?;
            return Ok(config);
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config from {:?}", path))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse config from {:?}", path))
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory {:?}", parent))?;
        }

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        fs::write(&path, content)
            .with_context(|| format!("Failed to write config to {:?}", path))?;

        Ok(())
    }

    /// Check if a system generation is pinned
    pub fn is_system_pinned(&self, gen_id: u32) -> bool {
        self.pinned.system.contains(&gen_id)
    }

    /// Check if a home-manager generation is pinned
    pub fn is_home_manager_pinned(&self, gen_id: u32) -> bool {
        self.pinned.home_manager.contains(&gen_id)
    }

    /// Toggle pin status for system generation
    pub fn toggle_system_pin(&mut self, gen_id: u32) {
        if self.pinned.system.contains(&gen_id) {
            self.pinned.system.remove(&gen_id);
        } else {
            self.pinned.system.insert(gen_id);
        }
    }

    /// Toggle pin status for home-manager generation
    pub fn toggle_home_manager_pin(&mut self, gen_id: u32) {
        if self.pinned.home_manager.contains(&gen_id) {
            self.pinned.home_manager.remove(&gen_id);
        } else {
            self.pinned.home_manager.insert(gen_id);
        }
    }
}

/// Available theme names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeName {
    #[default]
    Gruvbox,
    Nord,
    Transparent,
}

impl ThemeName {
    pub fn all() -> &'static [ThemeName] {
        &[ThemeName::Gruvbox, ThemeName::Nord, ThemeName::Transparent]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeName::Gruvbox => "Gruvbox",
            ThemeName::Nord => "Nord",
            ThemeName::Transparent => "Transparent",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            ThemeName::Gruvbox => ThemeName::Nord,
            ThemeName::Nord => ThemeName::Transparent,
            ThemeName::Transparent => ThemeName::Gruvbox,
        }
    }
}

/// Layout mode for the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    #[default]
    Auto,
    SideBySide,
    TabsOnly,
}

impl LayoutMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            LayoutMode::Auto => "Auto (responsive)",
            LayoutMode::SideBySide => "Side-by-side",
            LayoutMode::TabsOnly => "Tabs only",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            LayoutMode::Auto => LayoutMode::SideBySide,
            LayoutMode::SideBySide => LayoutMode::TabsOnly,
            LayoutMode::TabsOnly => LayoutMode::Auto,
        }
    }
}

/// Display options for generation info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayOptions {
    pub show_nixos_version: bool,
    pub show_kernel_version: bool,
    pub show_package_count: bool,
    pub show_size: bool,
    pub show_store_path: bool,
    pub show_boot_entry: bool,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            show_nixos_version: true,
            show_kernel_version: true,
            show_package_count: true,
            show_size: true,
            show_store_path: false,
            show_boot_entry: true,
        }
    }
}

/// Pinned generations (protected from deletion)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PinnedGenerations {
    #[serde(default)]
    pub system: HashSet<u32>,
    #[serde(default)]
    pub home_manager: HashSet<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.theme, ThemeName::Gruvbox);
        assert_eq!(config.layout, LayoutMode::Auto);
        assert!(config.display.show_nixos_version);
    }

    #[test]
    fn test_pin_toggle() {
        let mut config = Config::default();
        assert!(!config.is_system_pinned(42));
        
        config.toggle_system_pin(42);
        assert!(config.is_system_pinned(42));
        
        config.toggle_system_pin(42);
        assert!(!config.is_system_pinned(42));
    }

    #[test]
    fn test_theme_cycle() {
        let theme = ThemeName::Gruvbox;
        assert_eq!(theme.next(), ThemeName::Nord);
        assert_eq!(theme.next().next(), ThemeName::Transparent);
        assert_eq!(theme.next().next().next(), ThemeName::Gruvbox);
    }
}
