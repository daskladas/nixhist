//! System detection for NixOS and Home-Manager
//!
//! Detects:
//! - Whether the system uses Flakes or Channels
//! - Whether Home-Manager is installed (standalone or as NixOS module)
//! - Profile paths for both System and Home-Manager

use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;

/// Information about the detected system configuration
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub hostname: String,
    pub username: String,
    pub uses_flakes: bool,
    pub system_profile: PathBuf,
    pub home_manager: Option<HomeManagerInfo>,
}

/// Home-Manager installation info
#[derive(Debug, Clone)]
pub struct HomeManagerInfo {
    pub profile_path: PathBuf,
    pub is_standalone: bool,
}

/// Detect system configuration
/// 
/// This function checks for the presence of various Nix components
/// and returns information about how the system is configured.
pub fn detect_system() -> Result<SystemInfo> {
    let hostname = get_hostname()?;
    let username = get_username()?;
    let uses_flakes = detect_flakes();
    let system_profile = PathBuf::from("/nix/var/nix/profiles/system");
    let home_manager = detect_home_manager(&username);

    Ok(SystemInfo {
        hostname,
        username,
        uses_flakes,
        system_profile,
        home_manager,
    })
}

/// Get the system hostname
fn get_hostname() -> Result<String> {
    // Try /etc/hostname first
    if let Ok(hostname) = std::fs::read_to_string("/etc/hostname") {
        let hostname = hostname.trim().to_string();
        if !hostname.is_empty() {
            return Ok(hostname);
        }
    }

    // Fallback to hostname command
    let output = std::process::Command::new("hostname")
        .output()
        .context("Failed to get hostname")?;

    let hostname = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();

    if hostname.is_empty() {
        Ok("unknown".to_string())
    } else {
        Ok(hostname)
    }
}

/// Get the current username
fn get_username() -> Result<String> {
    env::var("USER")
        .or_else(|_| env::var("LOGNAME"))
        .context("Could not determine username from USER or LOGNAME environment variable")
}

/// Check if the system uses Flakes
fn detect_flakes() -> bool {
    // Check for flake.nix in common locations
    let home = env::var("HOME").unwrap_or_default();
    let flake_paths = [
        PathBuf::from("/etc/nixos/flake.nix"),
        PathBuf::from(&home).join(".config/nixos/flake.nix"),
        PathBuf::from(&home).join("nixos/flake.nix"),
        PathBuf::from(&home).join(".nixos/flake.nix"),
    ];

    flake_paths.iter().any(|p| p.exists())
}

/// Detect Home-Manager installation
/// 
/// Checks both standalone and NixOS module installations.
/// Returns None if Home-Manager is not detected.
fn detect_home_manager(username: &str) -> Option<HomeManagerInfo> {
    // Check standalone installation first (more common for Flake users)
    let home = env::var("HOME").ok()?;
    let standalone_path = PathBuf::from(&home)
        .join(".local/state/home-manager/profiles");

    if standalone_path.exists() && has_generation_links(&standalone_path) {
        return Some(HomeManagerInfo {
            profile_path: standalone_path,
            is_standalone: true,
        });
    }

    // Check NixOS module installation
    let module_path = PathBuf::from("/nix/var/nix/profiles/per-user")
        .join(username)
        .join("home-manager");

    // The module path is a symlink, check if it exists
    if module_path.exists() || module_path.is_symlink() {
        // For module installation, we need the directory containing the links
        let profile_dir = module_path.parent()?;
        return Some(HomeManagerInfo {
            profile_path: profile_dir.to_path_buf(),
            is_standalone: false,
        });
    }

    // Alternative standalone location (older Home-Manager)
    let alt_standalone = PathBuf::from(&home)
        .join(".nix-profile");
    
    // Check if this is actually a home-manager profile
    if alt_standalone.exists() {
        let alt_state = PathBuf::from(&home)
            .join(".local/state/nix/profiles/home-manager");
        if alt_state.exists() {
            return Some(HomeManagerInfo {
                profile_path: alt_state.parent()?.to_path_buf(),
                is_standalone: true,
            });
        }
    }

    None
}

/// Check if a directory contains generation links
fn has_generation_links(path: &PathBuf) -> bool {
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Look for home-manager-N-link pattern
            if name_str.starts_with("home-manager-") && name_str.ends_with("-link") {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_username() {
        // This should work in most environments
        let result = get_username();
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }
}
