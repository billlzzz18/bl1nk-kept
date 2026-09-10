//! Agent installation status and update preview types.

use serde::{Deserialize, Serialize};

/// Installation status of an external CLI agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusCheck {
    /// The agent binary is installed and reachable.
    Installed,
    /// The agent binary is not found on `$PATH`.
    NotInstalled,
    /// Installed but a newer bundled version is available.
    UpdateAvailable,
}

/// Preview of what an agent update would change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePreview {
    /// Human-readable agent label (e.g. "Claude Code").
    pub label: String,
    /// Currently installed version string.
    pub installed: String,
    /// Version bundled with this release of kept.
    pub bundled: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE-001: StatusCheck enum must round-trip through JSON for MCP transport
    #[test]
    fn status_check_serde_roundtrip() {
        let cases =
            [StatusCheck::Installed, StatusCheck::NotInstalled, StatusCheck::UpdateAvailable];
        for original in cases {
            let json = serde_json::to_string(&original).expect("serialize");
            let restored: StatusCheck = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(original, restored);
        }
    }

    #[test]
    fn status_check_equality() {
        assert_eq!(StatusCheck::Installed, StatusCheck::Installed);
        assert_ne!(StatusCheck::Installed, StatusCheck::NotInstalled);
        assert_ne!(StatusCheck::NotInstalled, StatusCheck::UpdateAvailable);
    }

    #[test]
    fn status_check_is_copy() {
        let a = StatusCheck::Installed;
        let b = a;
        assert_eq!(a, b);
    }

    // NOTE-002: UpdatePreview carries label + version pair for CLI display
    #[test]
    fn update_preview_serde_roundtrip() {
        let preview = UpdatePreview {
            label: "Claude Code".to_string(),
            installed: "1.0.0".to_string(),
            bundled: "1.1.0".to_string(),
        };
        let json = serde_json::to_string(&preview).expect("serialize");
        let restored: UpdatePreview = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.label, "Claude Code");
        assert_eq!(restored.installed, "1.0.0");
        assert_eq!(restored.bundled, "1.1.0");
    }

    #[test]
    fn update_preview_clone() {
        let preview = UpdatePreview {
            label: "test".to_string(),
            installed: "0.1".to_string(),
            bundled: "0.2".to_string(),
        };
        let cloned = preview.clone();
        assert_eq!(cloned.label, preview.label);
        assert_eq!(cloned.installed, preview.installed);
        assert_eq!(cloned.bundled, preview.bundled);
    }
}
