//! Agent lifecycle management subcommand.

use anyhow::Result;
use kept_agent::detect::AgentSetup;
use kept_agent::StatusCheck;

/// Subcommands for agent management.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum AgentSubcommand {
    /// Show status of all registered agents.
    Status,
    /// Install lifecycle hooks for an agent.
    Install {
        /// Agent name (e.g. "antigravity").
        name: String,
    },
    /// Remove lifecycle hooks for an agent.
    Uninstall {
        /// Agent name (e.g. "antigravity").
        name: String,
    },
    /// Check hook status for a specific agent.
    Check {
        /// Agent name (e.g. "antigravity").
        name: String,
    },
}

pub fn handle_agent(action: Option<AgentSubcommand>) -> Result<()> {
    let agents = kept_agent::detect::registered_agents();

    match action {
        None | Some(AgentSubcommand::Status) => {
            for agent in &agents {
                let detected = agent
                    .detect()
                    .map(|r| r.to_string())
                    .unwrap_or_else(|| "not found".to_string());
                let status = agent.check()?;
                let status_str = match status {
                    StatusCheck::Installed => "installed",
                    StatusCheck::NotInstalled => "not installed",
                    StatusCheck::UpdateAvailable => "update available",
                };
                println!("{:<20} detected: {:<30} hooks: {}", agent.name(), detected, status_str);
            }
            Ok(())
        }
        Some(AgentSubcommand::Install { name }) => {
            let agent = find_agent(&agents, &name)?;
            println!("{}", agent.install()?);
            Ok(())
        }
        Some(AgentSubcommand::Uninstall { name }) => {
            let agent = find_agent(&agents, &name)?;
            println!("{}", agent.uninstall()?);
            Ok(())
        }
        Some(AgentSubcommand::Check { name }) => {
            let agent = find_agent(&agents, &name)?;
            match agent.check()? {
                StatusCheck::Installed => println!("{}: installed", agent.name()),
                StatusCheck::NotInstalled => println!("{}: not installed", agent.name()),
                StatusCheck::UpdateAvailable => println!("{}: update available", agent.name()),
            }
            Ok(())
        }
    }
}

fn find_agent<'a>(agents: &'a [Box<dyn AgentSetup>], name: &str) -> Result<&'a dyn AgentSetup> {
    agents
        .iter()
        .find(|a| a.name() == name)
        .map(|b| b.as_ref())
        .ok_or_else(|| anyhow::anyhow!("Unknown agent: {}", name))
}
