//! Virtual terminal grid & ANSI engine for TUI execution in bl1nk-kept.
//! NOTE-TERM: Option 2 Full Virtual Grid Emulation using alacritty_terminal & vte.

pub mod alacritty;
pub mod pty_info;
pub mod runner;
pub mod terminal_settings;

use std::cmp;
use std::ops::Range as StdRange;
use std::sync::Arc;

use alacritty_terminal::vte::ansi::{Attr, Handler, Processor, StdSyncHandler};
pub use alacritty_terminal::vte::ansi::{Color, NamedColor, Rgb};
use serde::{Deserialize, Serialize};

pub use self::alacritty::{
    AlacrittyCell, AlacrittyTerm, AlacrittyTermConfig, AlacrittyTermLock, clear_saved_screen,
    content_lines, content_text, default_term_config, history_size, new_term, resize, screen_lines,
    total_lines,
};
pub use self::runner::{
    DEFAULT_COLUMNS, DEFAULT_HISTORY_LINES, DEFAULT_LINES, TIMEOUT_EXIT_CODE, TerminalRunOptions,
    TerminalRunRequest, TerminalRunResult, TerminalSink, TerminalStreamSnapshot,
    render_terminal_bytes, run_terminal_command,
};
pub use self::terminal_settings::{AlternateScroll, CursorShape, TerminalSettings};

pub type AnsiSpans = Vec<(StdRange<usize>, Option<Color>)>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ParsedAnsiText {
    pub text: String,
    pub foreground_spans: AnsiSpans,
    pub background_spans: AnsiSpans,
}

pub fn parse_ansi_text(input: &[u8]) -> ParsedAnsiText {
    let mut handler = StyledAnsiTextHandler::default();
    let mut processor = Processor::<StdSyncHandler>::default();
    for &byte in input {
        processor.advance(&mut handler, byte);
    }
    handler.finish()
}

pub fn strip_ansi_text(input: &[u8]) -> String {
    let mut handler = PlainAnsiTextHandler::default();
    let mut processor = Processor::<StdSyncHandler>::default();
    for &byte in input {
        processor.advance(&mut handler, byte);
    }
    handler.text
}

#[derive(Default)]
struct StyledAnsiTextHandler {
    text: String,
    foreground_spans: AnsiSpans,
    background_spans: AnsiSpans,
    current_foreground_range_start: usize,
    current_background_range_start: usize,
    current_foreground_color: Option<Color>,
    current_background_color: Option<Color>,
}

impl StyledAnsiTextHandler {
    fn finish(mut self) -> ParsedAnsiText {
        if self.current_foreground_range_start < self.text.len() {
            self.foreground_spans.push((
                self.current_foreground_range_start..self.text.len(),
                self.current_foreground_color,
            ));
        }

        if self.current_background_range_start < self.text.len() {
            self.background_spans.push((
                self.current_background_range_start..self.text.len(),
                self.current_background_color,
            ));
        }

        ParsedAnsiText {
            text: self.text,
            foreground_spans: self.foreground_spans,
            background_spans: self.background_spans,
        }
    }

    fn break_foreground_span(&mut self, color: Option<Color>) {
        if self.current_foreground_range_start < self.text.len() {
            self.foreground_spans.push((
                self.current_foreground_range_start..self.text.len(),
                self.current_foreground_color,
            ));
        }
        self.current_foreground_color = color;
        self.current_foreground_range_start = self.text.len();
    }

    fn break_background_span(&mut self, color: Option<Color>) {
        if self.current_background_range_start < self.text.len() {
            self.background_spans.push((
                self.current_background_range_start..self.text.len(),
                self.current_background_color,
            ));
        }
        self.current_background_color = color;
        self.current_background_range_start = self.text.len();
    }
}

impl Handler for StyledAnsiTextHandler {
    fn input(&mut self, c: char) {
        self.text.push(c);
    }

    fn linefeed(&mut self) {
        self.text.push('\n');
    }

    fn put_tab(&mut self, count: u16) {
        self.text.extend(std::iter::repeat_n('\t', count as usize));
    }

    fn terminal_attribute(&mut self, attr: Attr) {
        match attr {
            Attr::Foreground(color) => {
                self.break_foreground_span(Some(color));
            }
            Attr::Background(color) => {
                self.break_background_span(Some(color));
            }
            Attr::Reset => {
                self.break_foreground_span(None);
                self.break_background_span(None);
            }
            _ => {}
        }
    }
}

#[derive(Default)]
struct PlainAnsiTextHandler {
    text: String,
    line_start: usize,
}

impl Handler for PlainAnsiTextHandler {
    fn input(&mut self, c: char) {
        self.text.push(c);
    }

    fn linefeed(&mut self) {
        self.text.push('\n');
        self.line_start = self.text.len();
    }

    fn carriage_return(&mut self) {
        self.text.truncate(self.line_start);
    }

    fn put_tab(&mut self, count: u16) {
        self.text.extend(std::iter::repeat_n('\t', count as usize));
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub line: i32,
    pub column: usize,
}

impl Point {
    pub const fn new(line: i32, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalBounds {
    pub columns: usize,
    pub lines: usize,
}

impl TerminalBounds {
    pub const fn new(columns: usize, lines: usize) -> Self {
        Self { columns, lines }
    }

    pub fn num_columns(&self) -> usize {
        cmp::max(1, self.columns)
    }

    pub fn num_lines(&self) -> usize {
        cmp::max(1, self.lines)
    }
}

/// Virtual Terminal Session holding 2D grid and ANSI processor
pub struct Terminal {
    term: Arc<AlacrittyTermLock>,
    bounds: TerminalBounds,
    processor: Processor<StdSyncHandler>,
}

impl Terminal {
    pub fn new(bounds: TerminalBounds, history_lines: usize) -> Self {
        let config = default_term_config(history_lines);
        let term = new_term(&config, bounds);
        Self {
            term,
            bounds,
            processor: Processor::<StdSyncHandler>::new(),
        }
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        let mut term = self.term.lock();
        for &byte in bytes {
            self.processor.advance(&mut *term, byte);
        }
    }

    pub fn get_text(&self) -> String {
        content_text(&self.term)
    }

    pub fn resize(&mut self, new_bounds: TerminalBounds) {
        self.bounds = new_bounds;
        resize(&self.term, new_bounds);
    }

    pub fn clear(&mut self) {
        clear_saved_screen(&self.term);
    }

    pub fn cursor_position(&self) -> Point {
        let term = self.term.lock();
        let point = term.grid().cursor.point;
        Point::new(point.line.0, point.column.0)
    }

    pub fn total_lines(&self) -> usize {
        total_lines(&self.term)
    }

    pub fn viewport_lines(&self) -> usize {
        screen_lines(&self.term)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi_text() {
        let input = b"\x1b[31mHello\x1b[0m World";
        assert_eq!(strip_ansi_text(input), "Hello World");
    }

    #[test]
    fn test_parse_ansi_text() {
        let input = b"\x1b[31mRed\x1b[0m";
        let parsed = parse_ansi_text(input);
        assert_eq!(parsed.text, "Red");
        assert_eq!(parsed.foreground_spans.len(), 1);
    }

    #[test]
    fn test_virtual_terminal_grid() {
        let mut term = Terminal::new(TerminalBounds::new(80, 24), 100);
        term.write_bytes(b"hello world\r\nsecond line");
        let content = term.get_text();
        assert!(content.contains("hello world"));
        assert!(content.contains("second line"));
    }
}
