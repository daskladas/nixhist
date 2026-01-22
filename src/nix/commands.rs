//! Command execution for restore and delete operations
//!
//! Handles executing Nix commands with proper error handling.
//! Supports dry-run mode for safe testing.

use crate::types::ProfileType;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

/// Result of a command execution
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub success: bool,
    pub message: String,
    pub command: String,
}

/// Restore (switch to) a specific generation
pub fn restore_generation(
    profile_path: &Path,
    generation_id: u32,
    profile_type: ProfileType,
    dry_run: bool,
) -> Result<CommandResult> {
    let command = build_restore_command(profile_path, generation_id, profile_type);
    
    if dry_run {
        return Ok(CommandResult {
            success: true,
            message: format!("Dry run: Would execute restore to generation {}", generation_id),
            command,
        });
    }

    execute_sudo_command(&command, &format!("restore generation {}", generation_id))
}

/// Delete one or more generations
pub fn delete_generations(
    profile_path: &Path,
    generation_ids: &[u32],
    profile_type: ProfileType,
    dry_run: bool,
) -> Result<CommandResult> {
    if generation_ids.is_empty() {
        return Ok(CommandResult {
            success: false,
            message: "No generations specified for deletion".to_string(),
            command: String::new(),
        });
    }

    let command = build_delete_command(profile_path, generation_ids, profile_type);
    
    if dry_run {
        return Ok(CommandResult {
            success: true,
            message: format!("Dry run: Would delete {} generation(s)", generation_ids.len()),
            command,
        });
    }

    execute_sudo_command(&command, &format!("delete {} generation(s)", generation_ids.len()))
}

/// Build the restore command string
fn build_restore_command(
    profile_path: &Path,
    generation_id: u32,
    profile_type: ProfileType,
) -> String {
    match profile_type {
        ProfileType::System => {
            // For system, we need to switch-to-configuration
            let gen_path = profile_path
                .parent()
                .unwrap_or(Path::new("/nix/var/nix/profiles"))
                .join(format!("system-{}-link", generation_id));
            
            format!(
                "sudo {}/bin/switch-to-configuration switch",
                gen_path.display()
            )
        }
        ProfileType::HomeManager => {
            // For home-manager, activate the generation
            let home = std::env::var("HOME").unwrap_or_default();
            let gen_path = format!(
                "{}/.local/state/home-manager/profiles/home-manager-{}-link",
                home, generation_id
            );
            
            // Check if standalone or module
            if Path::new(&gen_path).exists() {
                format!("{}/activate", gen_path)
            } else {
                // Module installation - use nix-env
                format!(
                    "nix-env --switch-generation {} --profile {}",
                    generation_id,
                    profile_path.display()
                )
            }
        }
    }
}

/// Build the delete command string
fn build_delete_command(
    profile_path: &Path,
    generation_ids: &[u32],
    profile_type: ProfileType,
) -> String {
    let ids_str: Vec<String> = generation_ids.iter().map(|id| id.to_string()).collect();
    let ids_joined = ids_str.join(" ");

    match profile_type {
        ProfileType::System => {
            format!(
                "sudo nix-env --delete-generations {} --profile {}",
                ids_joined,
                profile_path.display()
            )
        }
        ProfileType::HomeManager => {
            // Check if home-manager command is available
            if command_exists("home-manager") {
                format!("home-manager remove-generations {}", ids_joined)
            } else {
                format!(
                    "nix-env --delete-generations {} --profile {}",
                    ids_joined,
                    profile_path.display()
                )
            }
        }
    }
}

/// Check if a command exists in PATH
fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Execute a command that may require sudo
fn execute_sudo_command(command: &str, description: &str) -> Result<CommandResult> {
    // Split command into parts
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        anyhow::bail!("Empty command");
    }

    let (program, args) = if parts[0] == "sudo" {
        ("sudo", &parts[1..])
    } else {
        (parts[0], &parts[1..])
    };

    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())  // Allow password input for sudo
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("Failed to execute: {}", command))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(CommandResult {
            success: true,
            message: format!("Successfully {}", description),
            command: command.to_string(),
        })
    } else {
        let error_msg = if !stderr.is_empty() {
            stderr.trim().to_string()
        } else if !stdout.is_empty() {
            stdout.trim().to_string()
        } else {
            format!("Command failed with exit code: {:?}", output.status.code())
        };

        Ok(CommandResult {
            success: false,
            message: format!("Failed to {}: {}", description, error_msg),
            command: command.to_string(),
        })
    }
}

/// Get the command that would be executed for restore (for display in confirmation)
pub fn get_restore_command_preview(
    profile_path: &Path,
    generation_id: u32,
    profile_type: ProfileType,
) -> String {
    build_restore_command(profile_path, generation_id, profile_type)
}

/// Get the command that would be executed for delete (for display in confirmation)
pub fn get_delete_command_preview(
    profile_path: &Path,
    generation_ids: &[u32],
    profile_type: ProfileType,
) -> String {
    build_delete_command(profile_path, generation_ids, profile_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_build_delete_command() {
        let path = PathBuf::from("/nix/var/nix/profiles/system");
        let cmd = build_delete_command(&path, &[140, 141], ProfileType::System);
        assert!(cmd.contains("sudo"));
        assert!(cmd.contains("--delete-generations"));
        assert!(cmd.contains("140"));
        assert!(cmd.contains("141"));
    }

    #[test]
    fn test_dry_run_restore() {
        let path = PathBuf::from("/nix/var/nix/profiles/system");
        let result = restore_generation(&path, 140, ProfileType::System, true).unwrap();
        assert!(result.success);
        assert!(result.message.contains("Dry run"));
    }

    #[test]
    fn test_dry_run_delete() {
        let path = PathBuf::from("/nix/var/nix/profiles/system");
        let result = delete_generations(&path, &[140, 141], ProfileType::System, true).unwrap();
        assert!(result.success);
        assert!(result.message.contains("Dry run"));
    }
}
