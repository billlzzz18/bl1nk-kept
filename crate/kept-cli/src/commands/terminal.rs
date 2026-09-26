//! Terminal runner command handlers backed by the kept-core virtual terminal engine.

use std::path::PathBuf;

use clap::Subcommand;

use kept_core::terminal::runner::{
    DEFAULT_COLUMNS, DEFAULT_HISTORY_LINES, DEFAULT_LINES, TerminalRunOptions, TerminalRunRequest,
    TerminalStreamSnapshot, render_terminal_bytes, run_terminal_command,
};

#[derive(Subcommand)]
pub enum TerminalCommands {
    /// Run a program inside a headless virtual terminal and render its screen.
    ///
    /// Runner options (`--columns`, `--lines`, `--history`, `--timeout-ms`,
    /// `--cwd`, `--json`) must appear before `<program>`; everything after the
    /// program is passed through to the child untouched.
    Run {
        /// Executable to run.
        program: String,
        /// Arguments passed to the executable.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        /// Working directory for the child process.
        #[arg(long)]
        cwd: Option<PathBuf>,
        /// Viewport width in character cells.
        #[arg(long, default_value_t = DEFAULT_COLUMNS)]
        columns: usize,
        /// Viewport height in character cells.
        #[arg(long, default_value_t = DEFAULT_LINES)]
        lines: usize,
        /// Scrollback rows kept behind the viewport.
        #[arg(long = "history", default_value_t = DEFAULT_HISTORY_LINES)]
        history_lines: usize,
        /// Kill the process after this many milliseconds.
        #[arg(long = "timeout-ms")]
        timeout_ms: Option<u64>,
        #[arg(short, long)]
        json: bool,
    },
    /// Render an already captured ANSI byte stream into a clean screen snapshot.
    Render {
        /// File holding the raw terminal byte stream.
        input: PathBuf,
        /// Viewport width in character cells.
        #[arg(long, default_value_t = DEFAULT_COLUMNS)]
        columns: usize,
        /// Viewport height in character cells.
        #[arg(long, default_value_t = DEFAULT_LINES)]
        lines: usize,
        /// Scrollback rows kept behind the viewport.
        #[arg(long = "history", default_value_t = DEFAULT_HISTORY_LINES)]
        history_lines: usize,
        #[arg(short, long)]
        json: bool,
    },
}

/// Handle `kept terminal`.
///
/// `kept terminal run` mirrors the child exit code, so scripts can chain
/// commands the same way they would with the wrapped program itself. A run that
/// hits its `--timeout-ms` deadline exits with `124`, matching `timeout(1)`.
pub fn handle_terminal(cmd: TerminalCommands) -> anyhow::Result<()> {
    match cmd {
        TerminalCommands::Run {
            program,
            args,
            cwd,
            columns,
            lines,
            history_lines,
            timeout_ms,
            json,
        } => {
            let options = TerminalRunOptions {
                columns,
                lines,
                history_lines,
                timeout_ms,
            };
            let request = TerminalRunRequest {
                program,
                args,
                working_directory: cwd,
                environment: Vec::new(),
                stdin: None,
            };

            let result = run_terminal_command(&request, &options)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("program: {}", result.program);
                println!("args: {}", result.args.join(" "));
                println!("exit_code: {}", result.exit_code.unwrap_or(1));
                println!("duration_ms: {}", result.duration_ms);
                println!("timed_out: {}", result.timed_out);
                print_stream("stdout", &result.stdout);
                print_stream("stderr", &result.stderr);
            }

            let exit_code = result.exit_code.unwrap_or(1);
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
            Ok(())
        }
        TerminalCommands::Render {
            input,
            columns,
            lines,
            history_lines,
            json,
        } => {
            let options = TerminalRunOptions {
                columns,
                lines,
                history_lines,
                timeout_ms: None,
            };
            let bytes = std::fs::read(&input)?;

            let snapshot = render_terminal_bytes(&bytes, &options);
            if json {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            } else {
                println!("bytes: {}", snapshot.bytes);
                println!("truncated: {}", snapshot.truncated);
                print_stream("screen", &snapshot);
            }
            Ok(())
        }
    }
}

fn print_stream(label: &str, stream: &TerminalStreamSnapshot) {
    if stream.screen.is_empty() {
        println!("{label}: <empty>");
        return;
    }

    println!("{label} ({} lines, {} bytes):", stream.lines, stream.bytes);
    println!("{}", stream.screen);
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    #[command(name = "kept")]
    struct TerminalOnlyCli {
        #[command(subcommand)]
        command: TerminalCommands,
    }

    #[test]
    fn run_parses_program_arguments_after_flags() {
        let cli = TerminalOnlyCli::try_parse_from([
            "kept",
            "run",
            "--columns",
            "100",
            "cargo",
            "test",
            "--workspace",
        ])
        .expect("terminal run must parse");

        match cli.command {
            TerminalCommands::Run { program, args, columns, .. } => {
                assert_eq!(program, "cargo");
                assert_eq!(columns, 100);
                assert_eq!(args, vec!["test".to_string(), "--workspace".to_string()]);
            }
            TerminalCommands::Render { .. } => panic!("expected the run subcommand"),
        }
    }

    #[test]
    fn render_parses_an_input_path() {
        let cli = TerminalOnlyCli::try_parse_from(["kept", "render", "capture.bin", "--json"])
            .expect("terminal render must parse");

        match cli.command {
            TerminalCommands::Render { input, json, .. } => {
                assert_eq!(input, PathBuf::from("capture.bin"));
                assert!(json);
            }
            TerminalCommands::Run { .. } => panic!("expected the render subcommand"),
        }
    }
}
