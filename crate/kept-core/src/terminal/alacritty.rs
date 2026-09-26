//! alacritty_terminal adapter layer: 2D virtual grid, buffer extraction, resize.
// NOTE-TERM-001: ใช้ `FairMutex` ของ alacritty_terminal แทน `parking_lot::Mutex` เพราะ
// `alacritty_terminal::event_loop::EventLoop` บังคับ signature `Arc<FairMutex<Term<U>>>`
// ถ้าใช้ mutex คนละชนิด โหมด PTY จะต่อ EventLoop ไม่ได้ และต้องคัดลอก grid ทั้งก้อน

use std::sync::Arc;

use alacritty_terminal::event::EventListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::cell::Cell;
use alacritty_terminal::term::{Config, Term};

use super::TerminalBounds;

impl Dimensions for TerminalBounds {
    fn total_lines(&self) -> usize {
        self.num_lines()
    }

    fn screen_lines(&self) -> usize {
        self.num_lines()
    }

    fn columns(&self) -> usize {
        self.num_columns()
    }
}

/// Headless event sink: the grid needs an [`EventListener`] but has no UI to notify.
#[derive(Clone, Default)]
pub struct EventProxy;

impl EventListener for EventProxy {}

pub type AlacrittyTerm = Term<EventProxy>;
pub type AlacrittyTermConfig = Config;
pub type AlacrittyTermLock = FairMutex<AlacrittyTerm>;
pub type AlacrittyCell = Cell;

pub fn new_term(config: &AlacrittyTermConfig, bounds: TerminalBounds) -> Arc<AlacrittyTermLock> {
    let term = Term::new(config.clone(), &bounds, EventProxy);
    Arc::new(FairMutex::new(term))
}

pub fn default_term_config(history: usize) -> AlacrittyTermConfig {
    Config {
        scrolling_history: history,
        ..Default::default()
    }
}

pub fn clear_saved_screen<U: EventListener>(term: &FairMutex<Term<U>>) {
    term.lock().grid_mut().reset();
}

/// Read every row of the buffer as text: scrollback first, then the viewport.
///
/// Trailing blank cells of each row are trimmed, so a fixed-width grid does not
/// leak padding spaces into the returned text.
pub fn content_lines<U: EventListener>(term: &FairMutex<Term<U>>) -> Vec<String> {
    let guard = term.lock();
    let grid = guard.grid();
    // NOTE-TERM-002: เดินด้วย Line index ตรง ๆ จาก topmost_line() (ติดลบสุดที่ grid ยอมรับ)
    // ถึง bottommost_line() จึงได้ทั้ง scrollback และ viewport โดยไม่ panic จาก debug_assert ของ Storage
    // และไม่ใช้ GridIterator เพราะ `Iterator::next()` ของมันเลื่อน point ก่อน yield
    // ทำให้ cell แรกของบรรทัดแรกหายไป
    let topmost = grid.topmost_line().0;
    let bottommost = grid.bottommost_line().0;
    let column_count = grid.columns();
    let mut rows: Vec<String> = Vec::with_capacity(grid.total_lines());

    for line in topmost..=bottommost {
        let mut row = String::with_capacity(column_count);
        for column in 0..column_count {
            row.push(grid[Point::new(Line(line), Column(column))].c);
        }
        rows.push(row.trim_end().to_string());
    }

    rows
}

/// Whole-buffer text: one line per grid row, scrollback included.
pub fn content_text<U: EventListener>(term: &FairMutex<Term<U>>) -> String {
    content_lines(term).join("\n")
}

pub fn resize<U: EventListener>(term: &FairMutex<Term<U>>, bounds: TerminalBounds) {
    term.lock().resize(bounds);
}

pub fn screen_lines<U: EventListener>(term: &FairMutex<Term<U>>) -> usize {
    term.lock().grid().screen_lines()
}

pub fn total_lines<U: EventListener>(term: &FairMutex<Term<U>>) -> usize {
    term.lock().grid().total_lines()
}

/// Number of scrollback rows currently retained behind the viewport.
pub fn history_size<U: EventListener>(term: &FairMutex<Term<U>>) -> usize {
    term.lock().grid().history_size()
}

pub fn columns<U: EventListener>(term: &FairMutex<Term<U>>) -> usize {
    term.lock().grid().columns()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::vte::ansi::{Processor, StdSyncHandler};

    #[test]
    fn test_content_extraction() {
        let config = default_term_config(100);
        let term = new_term(&config, TerminalBounds::new(80, 24));
        let mut processor = Processor::<StdSyncHandler>::new();
        let bytes = b"hello world\r\nsecond line";
        {
            let mut guard = term.lock();
            for &b in bytes {
                processor.advance(&mut *guard, b);
            }
        }
        let lines = content_lines(&term);
        assert_eq!(lines[0], "hello world");
        assert_eq!(lines[1], "second line");
        assert!(content_text(&term).contains("hello world\nsecond line"));
    }
}
