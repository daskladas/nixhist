//! Core data types for nixhist
//! 
//! This module defines all shared data structures used throughout the application.

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Represents a NixOS or Home-Manager generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generation {
    pub id: u32,
    pub date: DateTime<Local>,
    pub is_current: bool,
    pub nixos_version: Option<String>,
    pub kernel_version: Option<String>,
    pub package_count: usize,
    pub closure_size: u64,
    pub store_path: String,
    pub is_pinned: bool,
    pub in_bootloader: bool,
}

impl Generation {
    /// Format the date for display
    pub fn formatted_date(&self) -> String {
        self.date.format("%d.%m.%y %H:%M").to_string()
    }

    /// Format the closure size for display
    pub fn formatted_size(&self) -> String {
        format_bytes(self.closure_size)
    }
}

/// Represents a package in a generation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub size: u64,
}

impl Package {
    /// Format the size for display
    pub fn formatted_size(&self) -> String {
        format_bytes(self.size)
    }
}

/// Result of comparing two generations
#[derive(Debug, Clone, Default)]
pub struct GenerationDiff {
    pub added: Vec<Package>,
    pub removed: Vec<Package>,
    pub updated: Vec<PackageUpdate>,
}

impl GenerationDiff {
    /// Calculate diff between two package sets
    pub fn calculate(old_packages: &[Package], new_packages: &[Package]) -> Self {
        let old_set: HashSet<&str> = old_packages.iter().map(|p| p.name.as_str()).collect();
        let new_set: HashSet<&str> = new_packages.iter().map(|p| p.name.as_str()).collect();

        let added: Vec<Package> = new_packages
            .iter()
            .filter(|p| !old_set.contains(p.name.as_str()))
            .cloned()
            .collect();

        let removed: Vec<Package> = old_packages
            .iter()
            .filter(|p| !new_set.contains(p.name.as_str()))
            .cloned()
            .collect();

        // Find updated packages (same name, different version)
        let mut updated = Vec::new();
        for new_pkg in new_packages {
            if let Some(old_pkg) = old_packages.iter().find(|p| p.name == new_pkg.name) {
                if old_pkg.version != new_pkg.version {
                    updated.push(PackageUpdate {
                        name: new_pkg.name.clone(),
                        old_version: old_pkg.version.clone(),
                        new_version: new_pkg.version.clone(),
                        is_kernel: new_pkg.name.starts_with("linux-"),
                        is_security: is_security_package(&new_pkg.name),
                    });
                }
            }
        }

        Self { added, removed, updated }
    }

    /// Get summary string (e.g., "+8 -3 ~24")
    pub fn summary(&self) -> String {
        format!(
            "+{} added · -{} removed · ~{} updated",
            self.added.len(),
            self.removed.len(),
            self.updated.len()
        )
    }
}

/// Represents a package version update
#[derive(Debug, Clone)]
pub struct PackageUpdate {
    pub name: String,
    pub old_version: String,
    pub new_version: String,
    pub is_kernel: bool,
    pub is_security: bool,
}

/// Profile type (System or Home-Manager)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProfileType {
    System,
    HomeManager,
}

impl ProfileType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProfileType::System => "System",
            ProfileType::HomeManager => "Home-Manager",
        }
    }
}

/// Application tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Overview,
    Packages,
    Diff,
    Manage,
    Settings,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[Tab::Overview, Tab::Packages, Tab::Diff, Tab::Manage, Tab::Settings]
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Overview => 0,
            Tab::Packages => 1,
            Tab::Diff => 2,
            Tab::Manage => 3,
            Tab::Settings => 4,
        }
    }

    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => Tab::Overview,
            1 => Tab::Packages,
            2 => Tab::Diff,
            3 => Tab::Manage,
            4 => Tab::Settings,
            _ => Tab::Overview,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Packages => "Packages",
            Tab::Diff => "Diff",
            Tab::Manage => "Manage",
            Tab::Settings => "Settings",
        }
    }
}

// Helper functions

/// Format bytes to human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Check if a package is security-related
fn is_security_package(name: &str) -> bool {
    let security_packages = [
        "openssl", "openssh", "gnupg", "gpg", "sudo", "polkit",
        "pam", "shadow", "nss", "ca-certificates", "curl", "wget",
    ];
    security_packages.iter().any(|s| name.contains(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1500), "1.5 KB");
        assert_eq!(format_bytes(1_500_000), "1.4 MB");
        assert_eq!(format_bytes(1_500_000_000), "1.4 GB");
    }

    #[test]
    fn test_generation_diff() {
        let old = vec![
            Package { name: "foo".into(), version: "1.0".into(), size: 100 },
            Package { name: "bar".into(), version: "2.0".into(), size: 200 },
        ];
        let new = vec![
            Package { name: "foo".into(), version: "1.1".into(), size: 100 },
            Package { name: "baz".into(), version: "1.0".into(), size: 150 },
        ];

        let diff = GenerationDiff::calculate(&old, &new);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.updated.len(), 1);
    }
}
