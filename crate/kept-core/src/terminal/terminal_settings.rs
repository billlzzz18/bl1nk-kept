use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorShape {
    #[default]
    Block,
    Underline,
    Bar,
    Hollow,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlternateScroll {
    #[default]
    On,
    Off,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TerminalSettings {
    pub cursor_shape: CursorShape,
    pub alternate_scroll: AlternateScroll,
    pub open_links_in_mouse_mode: bool,
    pub shell: Option<String>,
}

impl TerminalSettings {
    pub fn get_global<T>(_cx: &T) -> Self {
        Self::default()
    }
}
