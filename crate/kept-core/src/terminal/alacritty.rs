use std::sync::Arc;
use alacritty_terminal::event::EventListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::cell::Cell;
use alacritty_terminal::term::{Config, Term};
use parking_lot::Mutex;

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

#[derive(Clone, Default)]
pub struct EventProxy;

impl EventListener for EventProxy {
    fn send_event(&self, _event: alacritty_terminal::event::Event) {}
}

pub type AlacrittyTerm = Term<EventProxy>;
pub type AlacrittyTermConfig = Config;
pub type AlacrittyTermLock = Mutex<AlacrittyTerm>;
pub type AlacrittyCell = Cell;

pub fn new_term(
    config: &AlacrittyTermConfig,
    bounds: TerminalBounds,
) -> Arc<AlacrittyTermLock> {
    let proxy = EventProxy;
    let term = Term::new(config.clone(), &bounds, proxy);
    Arc::new(Mutex::new(term))
}

pub fn default_term_config(
    history: usize,
) -> AlacrittyTermConfig {
    Config {
        scrolling_history: history,
        ..Default::default()
    }
}

pub fn clear_saved_screen(term: &AlacrittyTermLock) {
    let mut term = term.lock();
    term.grid_mut().reset();
}

pub fn content_text(term: &AlacrittyTermLock) -> String {
    let term = term.lock();
    let mut text = String::new();
    let grid = term.grid();
    for line in 0..grid.screen_lines() {
        for col in 0..grid.columns() {
            let point = Point::new(Line(line as i32), Column(col));
            let cell = &grid[point];
            text.push(cell.c);
        }
        text.push('\n');
    }
    text
}

pub fn resize(term: &AlacrittyTermLock, bounds: TerminalBounds) {
    term.lock().resize(bounds);
}

pub fn screen_lines(term: &AlacrittyTermLock) -> usize {
    term.lock().grid().screen_lines()
}

pub fn total_lines(term: &AlacrittyTermLock) -> usize {
    term.lock().grid().total_lines()
}
