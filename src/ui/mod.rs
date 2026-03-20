pub mod component;
pub mod types;

mod ui;
pub use ui::*;

pub mod prelude {
    pub use crate::state::State;
    pub use crate::types::*;
    pub use crate::ui::component::{Component, Hotkeys};
    pub use crate::ui::types::*;
    pub use crossterm::event::{Event, KeyCode, KeyEvent};
    pub use ratatui::prelude::*;
    pub use ratatui::widgets::{
        Block, BorderType, Borders, Clear, List, ListDirection, ListItem, ListState, Padding,
        Paragraph, Wrap,
    };
}
