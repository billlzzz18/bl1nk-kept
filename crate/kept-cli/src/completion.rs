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
