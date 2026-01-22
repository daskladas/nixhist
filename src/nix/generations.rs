//! Generation listing and parsing
//!
//! Handles listing generations for both System and Home-Manager profiles.
//! Parses generation metadata including version, kernel, size, etc.

use crate::types::{Generation, ProfileType};
use anyhow::{Context, Result};
use chrono::{DateTime, Local, TimeZone};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Source of generations (which profile)
#[derive(Debug, Clone)]
pub struct GenerationSource {
    pub profile_type: ProfileType,
    pub profile_path: PathBuf,
}

/// List all generations for a given profile
pub fn list_generations(source: &GenerationSource) -> Result<Vec<Generation>> {
    let profile_path = &source.profile_path;
    
    // Get generation list from nix-env
    let raw_generations = get_raw_generations(profile_path)?;
    
    // Get current generation ID
    let current_id = get_current_generation_id(profile_path)?;
    
    // Get boot entries (for system profile only)
    let boot_entries = if source.profile_type == ProfileType::System {
        get_boot_entries().unwrap_or_default()
    } else {
        Vec::new()
    };

    // Parse each generation
    let mut generations = Vec::new();
    for (id, timestamp) in raw_generations {
        let gen_path = get_generation_path(profile_path, id, source.profile_type);
        
        if !gen_path.exists() {
            continue; // Skip if path doesn't exist
        }

        let generation = parse_generation(
            id,
            timestamp,
            &gen_path,
            id == current_id,
            boot_entries.contains(&id),
            source.profile_type,
        )?;
        
        generations.push(generation);
    }

    // Sort by ID descending (newest first)
    generations.sort_by(|a, b| b.id.cmp(&a.id));

    Ok(generations)
}

/// Get raw generation list (ID and timestamp) from nix-env
fn get_raw_generations(profile_path: &Path) -> Result<Vec<(u32, DateTime<Local>)>> {
    let output = Command::new("nix-env")
        .args(["--list-generations", "--profile"])
        .arg(profile_path)
        .output()
        .context("Failed to run nix-env --list-generations")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("nix-env failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_generation_list(&stdout)
}

/// Parse nix-env --list-generations output
/// 
/// Example output:
///   1   2024-01-15 08:44:32
///   2   2024-01-18 11:03:15   (current)
fn parse_generation_list(output: &str) -> Result<Vec<(u32, DateTime<Local>)>> {
    let mut result = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Split by whitespace and parse
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }

        // Parse generation ID
        let id: u32 = parts[0].parse()
            .with_context(|| format!("Invalid generation ID: {}", parts[0]))?;

        // Parse date and time (parts[1] = date, parts[2] = time)
        let datetime_str = format!("{} {}", parts[1], parts[2]);
        let timestamp = parse_datetime(&datetime_str)?;

        result.push((id, timestamp));
    }

    Ok(result)
}

/// Parse datetime string (YYYY-MM-DD HH:MM:SS)
fn parse_datetime(s: &str) -> Result<DateTime<Local>> {
    let naive = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .with_context(|| format!("Failed to parse datetime: {}", s))?;
    
    Ok(Local.from_local_datetime(&naive)
        .single()
        .unwrap_or_else(|| Local::now()))
}

/// Get the current generation ID
fn get_current_generation_id(profile_path: &Path) -> Result<u32> {
    // The profile path is a symlink to the current generation
    let target = std::fs::read_link(profile_path)
        .with_context(|| format!("Failed to read profile symlink: {:?}", profile_path))?;

    extract_generation_id(&target)
}

/// Extract generation ID from a path like "system-142-link"
fn extract_generation_id(path: &Path) -> Result<u32> {
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .context("Invalid generation path")?;

    // Pattern: name-ID-link (e.g., "system-142-link" or "home-manager-89-link")
    let parts: Vec<&str> = filename.rsplitn(3, '-').collect();
    if parts.len() >= 2 && parts[0] == "link" {
        parts[1].parse()
            .with_context(|| format!("Invalid generation ID in path: {}", filename))
    } else {
        anyhow::bail!("Could not extract generation ID from: {}", filename)
    }
}

/// Get the path to a specific generation
fn get_generation_path(profile_path: &Path, id: u32, profile_type: ProfileType) -> PathBuf {
    let parent = profile_path.parent().unwrap_or(Path::new("/"));
    let name = match profile_type {
        ProfileType::System => format!("system-{}-link", id),
        ProfileType::HomeManager => format!("home-manager-{}-link", id),
    };
    parent.join(name)
}

/// Parse a single generation's metadata
fn parse_generation(
    id: u32,
    timestamp: DateTime<Local>,
    gen_path: &Path,
    is_current: bool,
    in_bootloader: bool,
    profile_type: ProfileType,
) -> Result<Generation> {
    // Get NixOS/HM version
    let nixos_version = get_version(gen_path, profile_type);
    
    // Get kernel version (system only)
    let kernel_version = if profile_type == ProfileType::System {
        get_kernel_version(gen_path)
    } else {
        None
    };

    // Get package count
    let package_count = get_package_count(gen_path);

    // Get closure size
    let closure_size = get_closure_size(gen_path).unwrap_or(0);

    // Get store path
    let store_path = std::fs::read_link(gen_path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(Generation {
        id,
        date: timestamp,
        is_current,
        nixos_version,
        kernel_version,
        package_count,
        closure_size,
        store_path,
        is_pinned: false, // Will be set by app based on config
        in_bootloader,
    })
}

/// Get NixOS or Home-Manager version
fn get_version(gen_path: &Path, profile_type: ProfileType) -> Option<String> {
    let version_file = match profile_type {
        ProfileType::System => gen_path.join("nixos-version"),
        ProfileType::HomeManager => gen_path.join("hm-version"),
    };

    if version_file.exists() {
        std::fs::read_to_string(&version_file)
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        // Try to extract from store path
        std::fs::read_link(gen_path).ok().and_then(|p| {
            let s = p.to_string_lossy();
            // Extract version from path like /nix/store/xxx-nixos-system-hostname-24.11...
            if let Some(idx) = s.find("-nixos-system-") {
                let rest = &s[idx + 14..];
                rest.split('-').nth(1).map(|v| v.to_string())
            } else {
                None
            }
        })
    }
}

/// Get kernel version from a generation
fn get_kernel_version(gen_path: &Path) -> Option<String> {
    let kernel_dir = gen_path.join("kernel");
    
    if kernel_dir.exists() {
        // Read the kernel path and extract version
        std::fs::read_link(&kernel_dir).ok().and_then(|p| {
            let s = p.to_string_lossy();
            // Extract version from path like /nix/store/xxx-linux-6.6.52/...
            for part in s.split('/') {
                if part.starts_with("linux-") && part.len() > 6 {
                    return Some(part[6..].split('-').next()?.to_string());
                }
            }
            None
        })
    } else {
        // Try kernel-modules
        let modules_dir = gen_path.join("kernel-modules/lib/modules");
        if modules_dir.exists() {
            std::fs::read_dir(&modules_dir).ok().and_then(|mut entries| {
                entries.next()?.ok().map(|e| {
                    e.file_name().to_string_lossy().to_string()
                })
            })
        } else {
            None
        }
    }
}

/// Get the number of packages in a generation
fn get_package_count(gen_path: &Path) -> usize {
    let sw_path = gen_path.join("sw/bin");
    
    if sw_path.exists() {
        // Count binaries as a rough estimate
        std::fs::read_dir(&sw_path)
            .map(|entries| entries.count())
            .unwrap_or(0)
    } else {
        // For home-manager, count from manifest
        let manifest = gen_path.join("home-files/.nix-profile/manifest.nix");
        if manifest.exists() {
            // This is a rough estimate
            std::fs::read_to_string(&manifest)
                .map(|s| s.matches("name = ").count())
                .unwrap_or(0)
        } else {
            0
        }
    }
}

/// Get the closure size of a generation
fn get_closure_size(gen_path: &Path) -> Result<u64> {
    let output = Command::new("nix")
        .args(["path-info", "-S"])
        .arg(gen_path)
        .output()
        .context("Failed to run nix path-info")?;

    if !output.status.success() {
        return Ok(0);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Output format: /nix/store/xxx-... 1234567890
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(size) = parts[1].parse::<u64>() {
                return Ok(size);
            }
        }
    }

    Ok(0)
}

/// Get list of generations in the bootloader
fn get_boot_entries() -> Result<Vec<u32>> {
    let mut entries = Vec::new();

    // Check systemd-boot entries
    let loader_entries = Path::new("/boot/loader/entries");
    if loader_entries.exists() {
        if let Ok(dir) = std::fs::read_dir(loader_entries) {
            for entry in dir.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                // Pattern: nixos-generation-142.conf
                if name_str.starts_with("nixos-generation-") && name_str.ends_with(".conf") {
                    let id_str = &name_str[17..name_str.len()-5];
                    if let Ok(id) = id_str.parse() {
                        entries.push(id);
                    }
                }
            }
        }
    }

    // Check GRUB entries (alternative)
    let grub_cfg = Path::new("/boot/grub/grub.cfg");
    if grub_cfg.exists() && entries.is_empty() {
        if let Ok(content) = std::fs::read_to_string(grub_cfg) {
            for line in content.lines() {
                // Look for menuentry lines with generation info
                if line.contains("NixOS") && line.contains("Generation") {
                    // Extract generation number
                    if let Some(start) = line.find("Generation ") {
                        let rest = &line[start + 11..];
                        let num: String = rest.chars()
                            .take_while(|c| c.is_ascii_digit())
                            .collect();
                        if let Ok(id) = num.parse() {
                            entries.push(id);
                        }
                    }
                }
            }
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_generation_list() {
        let input = r#"
   1   2024-01-15 08:44:32
   2   2024-01-18 11:03:15   (current)
  10   2024-01-22 14:32:00
"#;
        let result = parse_generation_list(input).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, 1);
        assert_eq!(result[1].0, 2);
        assert_eq!(result[2].0, 10);
    }

    #[test]
    fn test_extract_generation_id() {
        let path = PathBuf::from("system-142-link");
        assert_eq!(extract_generation_id(&path).unwrap(), 142);

        let path = PathBuf::from("home-manager-89-link");
        assert_eq!(extract_generation_id(&path).unwrap(), 89);
    }
}
