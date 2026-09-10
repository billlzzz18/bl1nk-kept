//! External CLI detection and agent lifecycle trait.

use crate::status::{StatusCheck, UpdatePreview};
use anyhow::Result;

/// Trait for agent lifecycle management.
///
/// Each external CLI agent (agy, etc.) implements this trait
/// to integrate with kept's agent management.
pub trait AgentSetup {
    /// Human-readable agent name.
    fn name(&self) -> &str;

    /// Detect if this agent is present on the system.
    ///
    /// Returns `Some(reason)` if detected, `None` otherwise.
    fn detect(&self) -> Option<&'static str>;

    /// Check if kept's lifecycle hooks are installed for this agent.
    fn check(&self) -> Result<StatusCheck>;

    /// Install lifecycle hooks for this agent.
    fn install(&self) -> Result<String>;

    /// Remove lifecycle hooks for this agent.
    fn uninstall(&self) -> Result<String>;

    /// Preview what an update would change, if any.
    fn update_preview(&self) -> Result<Option<UpdatePreview>>;
}

/// Return all registered agent implementations.
pub fn registered_agents() -> Vec<Box<dyn AgentSetup>> {
    vec![Box::new(crate::antigravity::Antigravity::new())]
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockAgent;

    impl AgentSetup for MockAgent {
        fn name(&self) -> &str {
            "mock-agent"
        }
        fn detect(&self) -> Option<&'static str> {
            Some("test override")
        }
        fn check(&self) -> Result<StatusCheck> {
            Ok(StatusCheck::Installed)
        }
        fn install(&self) -> Result<String> {
            Ok("installed".to_string())
        }
        fn uninstall(&self) -> Result<String> {
            Ok("uninstalled".to_string())
        }
        fn update_preview(&self) -> Result<Option<UpdatePreview>> {
            Ok(None)
        }
    }

    #[test]
    fn mock_agent_satisfies_trait() {
        let agent = MockAgent;
        assert_eq!(agent.name(), "mock-agent");
        assert!(agent.detect().is_some());
        assert!(matches!(agent.check().unwrap(), StatusCheck::Installed));
    }

    #[test]
    fn registered_agents_not_empty() {
        let agents = registered_agents();
        assert!(!agents.is_empty());
    }
}
