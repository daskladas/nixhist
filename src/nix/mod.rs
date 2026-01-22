//! Nix interaction layer
//!
//! This module handles all interactions with NixOS and Home-Manager:
//! - System detection (Flakes vs Channels, HM standalone vs module)
//! - Generation listing and parsing
//! - Package extraction
//! - Command execution (restore, delete)

pub mod detect;
pub mod generations;
pub mod packages;
pub mod commands;

pub use detect::{SystemInfo, detect_system};
pub use generations::{list_generations, GenerationSource};
pub use packages::get_packages;
pub use commands::{restore_generation, delete_generations, CommandResult};
