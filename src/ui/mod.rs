pub mod component;
pub mod types;

mod ui;
pub use ui::*;

pub mod prelude {
    pub use crate::state::State;
    pub use crate::types::*;
    pub use crate::ui::component::{Component, Hotkeys};
    pub use crate::ui::types::*;
    pub use crossterm::event::{Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind};
    pub use ratatui::prelude::*;
    pub use ratatui::symbols::scrollbar::Set as ScrollbarSet;
    pub use ratatui::text::ToSpan;
    pub use ratatui::widgets::{
        Block, BorderType, Borders, Clear, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
        Wrap,
    };
    pub use tui_input::Input as TuiInput;
    pub use tui_widget_list::{ListBuilder, ListState, ListView};
}
