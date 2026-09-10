//! Agent lifecycle management for external CLI integrations.

pub mod antigravity;
pub mod claude;
pub mod detect;
pub mod hooks;
pub mod json_config;
mod status;

pub use status::*;
