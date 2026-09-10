mod shells {
    pub use clap_complete::shells::{Bash, Fish, PowerShell, Zsh};
}

use clap_complete::Generator;

#[derive(Clone, Debug, clap::ValueEnum)]
#[non_exhaustive]
#[value(rename_all = "lower")]
pub enum Shell {
    Bash,
    Fish,
    PowerShell,
    Zsh,
}

impl Generator for Shell {
    fn file_name(&self, name: &str) -> String {
        match self {
            Shell::Bash => self::shells::Bash.file_name(name),
            Shell::Fish => self::shells::Fish.file_name(name),
            Shell::PowerShell => self::shells::PowerShell.file_name(name),
            Shell::Zsh => self::shells::Zsh.file_name(name),
        }
    }

    fn generate(&self, cmd: &clap::Command, buf: &mut dyn std::io::Write) {
        match self {
            Shell::Bash => self::shells::Bash.generate(cmd, buf),
            Shell::Fish => self::shells::Fish.generate(cmd, buf),
            Shell::PowerShell => self::shells::PowerShell.generate(cmd, buf),
            Shell::Zsh => self::shells::Zsh.generate(cmd, buf),
        }
    }
}

pub(crate) fn main(cmd: &mut clap::Command, shell: &Shell) {
    shell.generate(cmd, &mut std::io::stdout());
}

#[cfg(test)]
mod tests {
    use super::Shell;
    use clap_complete::Generator;

    #[test]
    fn shell_file_name_follows_clap_complete_convention() {
        assert_eq!(Shell::Bash.file_name("kept"), "kept.bash");
        assert_eq!(Shell::Fish.file_name("kept"), "kept.fish");
        assert_eq!(Shell::PowerShell.file_name("kept"), "_kept.ps1");
        assert_eq!(Shell::Zsh.file_name("kept"), "_kept");
    }

    #[test]
    fn shell_generate_writes_script_for_every_variant() {
        for shell in [Shell::Bash, Shell::Fish, Shell::PowerShell, Shell::Zsh] {
            let mut command = clap::Command::new("kept");
            let mut script = Vec::new();
            clap_complete::generate(shell.clone(), &mut command, "kept", &mut script);
            assert!(!script.is_empty(), "script must not be empty for {:?}", shell);
        }
    }
}
