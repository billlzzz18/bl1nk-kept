//! Headless terminal runner: drives the virtual grid from real process output.
//!
//! The runner answers one question for callers: *what does the screen look like
//! after this program finished?* Child output is pumped through the same ANSI
//! grid a terminal emulator would use, so carriage-return progress rewrites,
//! cursor addressing and alternate-screen output collapse into the final visible
//! text instead of leaking raw escape sequences.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use alacritty_terminal::vte::ansi::{Processor, StdSyncHandler};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use super::TerminalBounds;
use super::alacritty::{
    AlacrittyTermLock, columns, content_lines, default_term_config, history_size, new_term,
    total_lines,
};

/// Viewport width used when the caller does not choose one.
pub const DEFAULT_COLUMNS: usize = 80;
/// Viewport height used when the caller does not choose one.
pub const DEFAULT_LINES: usize = 24;
/// Scrollback capacity used when the caller does not choose one.
pub const DEFAULT_HISTORY_LINES: usize = 2_000;
/// Exit code reported when a run is killed because its deadline expired (mirrors `timeout(1)`).
pub const TIMEOUT_EXIT_CODE: i32 = 124;

/// How long a stream pump may take to drain after the child process exited.
const PUMP_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);
/// How often the runner polls the child process while waiting for it.
const CHILD_POLL_INTERVAL: Duration = Duration::from_millis(10);
/// Size of the buffer used to read a child stream.
const PUMP_BUFFER_BYTES: usize = 8 * 1024;

/// Geometry and deadline for one terminal run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalRunOptions {
    /// Viewport width in character cells.
    pub columns: usize,
    /// Viewport height in character cells.
    pub lines: usize,
    /// Number of scrollback rows kept behind the viewport.
    pub history_lines: usize,
    /// Wall-clock budget for the child process; `None` waits indefinitely.
    pub timeout_ms: Option<u64>,
}

impl Default for TerminalRunOptions {
    fn default() -> Self {
        Self {
            columns: DEFAULT_COLUMNS,
            lines: DEFAULT_LINES,
            history_lines: DEFAULT_HISTORY_LINES,
            timeout_ms: None,
        }
    }
}

impl TerminalRunOptions {
    /// Clamp the geometry so the grid always has at least one addressable cell.
    pub fn normalized(&self) -> Self {
        Self {
            columns: self.columns.max(1),
            lines: self.lines.max(1),
            history_lines: self.history_lines,
            timeout_ms: self.timeout_ms,
        }
    }

    /// Grid geometry derived from these options.
    pub fn bounds(&self) -> TerminalBounds {
        TerminalBounds::new(self.columns.max(1), self.lines.max(1))
    }
}

/// The child process to run.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalRunRequest {
    /// Executable name or path.
    pub program: String,
    /// Arguments passed to the executable.
    #[serde(default)]
    pub args: Vec<String>,
    /// Working directory for the child process.
    #[serde(default)]
    pub working_directory: Option<PathBuf>,
    /// Extra environment variables layered over the inherited environment.
    #[serde(default)]
    pub environment: Vec<(String, String)>,
    /// Text written to the child's stdin before the stream is closed.
    #[serde(default)]
    pub stdin: Option<String>,
}

/// Rendered state of one captured stream.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalStreamSnapshot {
    /// Rendered buffer with trailing blank rows removed.
    pub screen: String,
    /// Number of rows retained in [`Self::screen`].
    pub lines: usize,
    /// Total grid rows, scrollback included.
    pub total_lines: usize,
    /// Scrollback rows currently held behind the viewport.
    pub history_lines: usize,
    /// Viewport width in character cells.
    pub columns: usize,
    /// Bytes fed into the grid.
    pub bytes: usize,
    /// True when the stream outgrew the configured scrollback capacity.
    pub truncated: bool,
}

/// Outcome of one terminal run.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalRunResult {
    /// Executable that was run.
    pub program: String,
    /// Arguments passed to the executable.
    pub args: Vec<String>,
    /// Process exit code, or [`TIMEOUT_EXIT_CODE`] when the run timed out.
    pub exit_code: Option<i32>,
    /// True only when the process exited with code `0` inside the deadline.
    pub success: bool,
    /// True when the run was killed because its deadline expired.
    pub timed_out: bool,
    /// Wall-clock duration of the run in milliseconds.
    pub duration_ms: u64,
    /// Rendered standard output.
    pub stdout: TerminalStreamSnapshot,
    /// Rendered standard error.
    pub stderr: TerminalStreamSnapshot,
}

/// Thread-safe feed for a single virtual grid.
#[derive(Clone)]
pub struct TerminalSink {
    inner: Arc<Mutex<TerminalSinkState>>,
}

struct TerminalSinkState {
    term: Arc<AlacrittyTermLock>,
    processor: Processor<StdSyncHandler>,
    bytes: usize,
}

impl TerminalSink {
    /// Create a sink backed by a fresh grid of the given geometry.
    pub fn new(bounds: TerminalBounds, history_lines: usize) -> Self {
        let config = default_term_config(history_lines);
        Self {
            inner: Arc::new(Mutex::new(TerminalSinkState {
                term: new_term(&config, bounds),
                processor: Processor::<StdSyncHandler>::new(),
                bytes: 0,
            })),
        }
    }

    /// Feed raw terminal bytes into the grid.
    pub fn write(&self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }

        let mut state = self.inner.lock();
        let terminal = Arc::clone(&state.term);
        let processor = &mut state.processor;
        let mut guard = terminal.lock();
        for &byte in bytes {
            processor.advance(&mut *guard, byte);
        }
        drop(guard);
        state.bytes += bytes.len();
    }

    /// Render the current grid into a serializable snapshot.
    pub fn snapshot(&self, options: &TerminalRunOptions) -> TerminalStreamSnapshot {
        let state = self.inner.lock();
        let rows = content_lines(&state.term);
        let history = history_size(&state.term);

        let mut retained = rows.len();
        while retained > 0 && rows[retained - 1].is_empty() {
            retained -= 1;
        }

        TerminalStreamSnapshot {
            screen: rows[..retained].join("\n"),
            lines: retained,
            total_lines: total_lines(&state.term),
            history_lines: history,
            columns: columns(&state.term),
            bytes: state.bytes,
            truncated: options.history_lines > 0 && history >= options.history_lines,
        }
    }
}

/// Render a captured terminal byte stream without spawning a process.
pub fn render_terminal_bytes(bytes: &[u8], options: &TerminalRunOptions) -> TerminalStreamSnapshot {
    let options = options.normalized();
    let sink = TerminalSink::new(options.bounds(), options.history_lines);
    sink.write(bytes);
    sink.snapshot(&options)
}

/// Run a child process and render its output through a virtual terminal.
///
/// Standard output and standard error are captured into separate grids so the
/// rendered result stays deterministic; their arrival order is therefore not
/// preserved. `stdin` is written and closed immediately, so the child never
/// blocks waiting for input.
pub fn run_terminal_command(
    request: &TerminalRunRequest,
    options: &TerminalRunOptions,
) -> anyhow::Result<TerminalRunResult> {
    let program = request.program.trim();
    if program.is_empty() {
        anyhow::bail!("terminal run requires a non-empty program");
    }

    let options = options.normalized();
    let started = Instant::now();

    let mut command = Command::new(program);
    command.args(&request.args);
    if let Some(directory) = &request.working_directory {
        command.current_dir(directory);
    }
    for (key, value) in &request.environment {
        command.env(key, value);
    }
    command
        .stdin(if request.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|error| anyhow::anyhow!("failed to start '{program}': {error}"))?;

    // NOTE-TERM-003: เขียน stdin แล้วปิดทันที เพราะ process ที่รอ input จะค้างจนหมด deadline
    if let Some(input) = &request.stdin
        && let Some(mut handle) = child.stdin.take()
    {
        handle.write_all(input.as_bytes())?;
        drop(handle);
    }

    let stdout_sink = TerminalSink::new(options.bounds(), options.history_lines);
    let stderr_sink = TerminalSink::new(options.bounds(), options.history_lines);
    let pumps: Vec<Receiver<()>> = child
        .stdout
        .take()
        .map(|stream| spawn_stream_pump(stream, stdout_sink.clone()))
        .into_iter()
        .chain(
            child
                .stderr
                .take()
                .map(|stream| spawn_stream_pump(stream, stderr_sink.clone())),
        )
        .collect();

    let mut timed_out = false;
    let exit_status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }

        let expired = options
            .timeout_ms
            .is_some_and(|timeout_ms| started.elapsed() >= Duration::from_millis(timeout_ms));
        if expired {
            timed_out = true;
            if let Err(error) = child.kill() {
                log::debug!("terminal run could not kill '{program}': {error}");
            }
            break child.wait()?;
        }

        std::thread::sleep(CHILD_POLL_INTERVAL);
    };

    for pump in &pumps {
        if !wait_for_stream_pump(pump) {
            log::debug!("terminal stream pump for '{program}' did not drain within the deadline");
        }
    }

    let exit_code = if timed_out {
        Some(TIMEOUT_EXIT_CODE)
    } else {
        exit_status.code()
    };

    Ok(TerminalRunResult {
        program: request.program.clone(),
        args: request.args.clone(),
        exit_code,
        success: !timed_out && exit_code == Some(0),
        timed_out,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        stdout: stdout_sink.snapshot(&options),
        stderr: stderr_sink.snapshot(&options),
    })
}

/// Read one child stream to EOF, feeding every chunk into `sink`.
///
/// The returned channel reports completion, so the runner can bound how long it
/// waits for a stream a grandchild process might still hold open.
fn spawn_stream_pump<R>(mut reader: R, sink: TerminalSink) -> Receiver<()>
where
    R: Read + Send + 'static,
{
    let (done_tx, done_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buffer = [0_u8; PUMP_BUFFER_BYTES];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => sink.write(&buffer[..read]),
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    log::debug!("terminal stream read stopped: {error}");
                    break;
                }
            }
        }

        if done_tx.send(()).is_err() {
            log::debug!("terminal stream pump finished after the runner stopped waiting");
        }
    });
    done_rx
}

fn wait_for_stream_pump(pump: &Receiver<()>) -> bool {
    match pump.recv_timeout(PUMP_DRAIN_TIMEOUT) {
        Ok(()) => true,
        Err(error) => {
            log::debug!("terminal stream pump wait failed: {error}");
            false
        }
    }
}

#[cfg(all(test, windows))]
fn shell_request(script: &str) -> TerminalRunRequest {
    TerminalRunRequest {
        program: "cmd.exe".to_string(),
        args: vec!["/C".to_string(), script.to_string()],
        ..Default::default()
    }
}

#[cfg(all(test, not(windows)))]
fn shell_request(script: &str) -> TerminalRunRequest {
    TerminalRunRequest {
        program: "sh".to_string(),
        args: vec!["-c".to_string(), script.to_string()],
        ..Default::default()
    }
}

#[cfg(all(test, windows))]
fn print_script(text: &str, to_stderr: bool) -> String {
    if to_stderr {
        format!("echo {text} 1>&2")
    } else {
        format!("echo {text}")
    }
}

#[cfg(all(test, not(windows)))]
fn print_script(text: &str, to_stderr: bool) -> String {
    if to_stderr {
        format!("printf '{text}\\n' 1>&2")
    } else {
        format!("printf '{text}\\n'")
    }
}

#[cfg(all(test, windows))]
fn slow_script() -> String {
    "ping -n 20 127.0.0.1 > NUL".to_string()
}

#[cfg(all(test, not(windows)))]
fn slow_script() -> String {
    "sleep 20".to_string()
}

#[cfg(all(test, windows))]
fn stdin_echo_request() -> TerminalRunRequest {
    TerminalRunRequest {
        program: "cmd.exe".to_string(),
        args: vec!["/C".to_string(), "more".to_string()],
        stdin: Some("hello\n".to_string()),
        ..Default::default()
    }
}

#[cfg(all(test, not(windows)))]
fn stdin_echo_request() -> TerminalRunRequest {
    TerminalRunRequest {
        program: "cat".to_string(),
        stdin: Some("hello\n".to_string()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> TerminalRunOptions {
        TerminalRunOptions {
            columns: 40,
            lines: 6,
            history_lines: 100,
            timeout_ms: None,
        }
    }

    #[test]
    fn render_collapses_carriage_return_progress_rewrites() {
        let input = b"progress 10%\rprogress 100%\n";

        let snapshot = render_terminal_bytes(input, &options());

        assert_eq!(snapshot.screen, "progress 100%");
        assert_eq!(snapshot.lines, 1);
        assert_eq!(snapshot.bytes, input.len());
        assert_eq!(snapshot.columns, 40);
    }

    #[test]
    fn render_drops_escape_sequences_and_keeps_text() {
        let snapshot = render_terminal_bytes(b"\x1b[31mred\x1b[0m plain", &options());

        assert_eq!(snapshot.screen, "red plain");
    }

    #[test]
    fn render_keeps_scrollback_beyond_the_viewport_height() {
        let mut input = String::new();
        for index in 0..20 {
            input.push_str(&format!("line {index}\n"));
        }

        let snapshot = render_terminal_bytes(input.as_bytes(), &options());

        assert!(snapshot.screen.contains("line 19"), "{}", snapshot.screen);
        assert!(snapshot.screen.contains("line 0\n"), "{}", snapshot.screen);
        assert!(snapshot.history_lines > 0);
        assert!(snapshot.total_lines > snapshot.lines);
        assert!(!snapshot.truncated);
    }

    #[test]
    fn render_marks_truncation_when_scrollback_saturates() {
        let mut input = String::new();
        for index in 0..80 {
            input.push_str(&format!("line {index}\n"));
        }
        let narrow = TerminalRunOptions {
            columns: 40,
            lines: 3,
            history_lines: 5,
            timeout_ms: None,
        };

        let snapshot = render_terminal_bytes(input.as_bytes(), &narrow);

        assert!(snapshot.truncated);
        assert_eq!(snapshot.history_lines, 5);
        assert!(snapshot.screen.contains("line 79"), "{}", snapshot.screen);
        assert!(!snapshot.screen.contains("line 0\n"), "{}", snapshot.screen);
    }

    #[test]
    fn run_rejects_a_blank_program() {
        let request = TerminalRunRequest {
            program: "   ".to_string(),
            ..Default::default()
        };

        let error =
            run_terminal_command(&request, &options()).expect_err("blank program must be refused");

        assert!(error.to_string().contains("non-empty program"));
    }
    #[test]
    fn run_captures_stdout_and_a_zero_exit_code() {
        let request = shell_request(&print_script("terminal-runner", false));

        let result = run_terminal_command(&request, &options()).expect("terminal run must succeed");

        assert_eq!(result.exit_code, Some(0));
        assert!(result.success);
        assert!(!result.timed_out);
        assert_eq!(result.stdout.screen, "terminal-runner");
        assert!(result.stdout.bytes > 0);
        assert_eq!(result.stderr.screen, "");
        assert_eq!(result.program, request.program);
        assert_eq!(result.args, request.args);
    }

    #[test]
    fn run_keeps_standard_error_separate_from_standard_output() {
        let request = shell_request(&print_script("terminal-error", true));

        let result = run_terminal_command(&request, &options()).expect("terminal run must succeed");

        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.stdout.screen, "");
        assert_eq!(result.stderr.screen, "terminal-error");
    }

    #[test]
    fn run_reports_a_failing_exit_code() {
        let request = shell_request("exit 3");

        let result =
            run_terminal_command(&request, &options()).expect("terminal run must return a result");

        assert_eq!(result.exit_code, Some(3));
        assert!(!result.success);
        assert!(!result.timed_out);
    }

    #[test]
    fn run_writes_and_closes_standard_input() {
        let request = stdin_echo_request();
        let options = TerminalRunOptions {
            timeout_ms: Some(10_000),
            ..options()
        };

        let result = run_terminal_command(&request, &options).expect("terminal run must succeed");

        assert!(!result.timed_out);
        assert!(result.stdout.screen.contains("hello"), "{}", result.stdout.screen);
    }

    #[test]
    fn run_kills_the_child_when_the_deadline_expires() {
        let request = shell_request(&slow_script());
        let options = TerminalRunOptions {
            timeout_ms: Some(200),
            ..options()
        };

        let result = run_terminal_command(&request, &options).expect("terminal run must return");

        assert!(result.timed_out);
        assert_eq!(result.exit_code, Some(TIMEOUT_EXIT_CODE));
        assert!(!result.success);
    }
}
