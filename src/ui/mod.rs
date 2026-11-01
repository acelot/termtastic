pub mod helpers;
pub mod logo;
pub mod screen;
pub mod widget;

mod ui;
pub use ui::*;

#[allow(unused_imports)]
pub mod prelude {
    pub use crate::state::State;
    pub use crate::types::*;
    pub use crate::ui::helpers::*;
    pub use crate::ui::screen::Screen;
    pub use crate::ui::widget::*;
    pub use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind};
    pub use ratatui::layout::Flex;
    pub use ratatui::prelude::*;
    pub use ratatui::text::{Text, ToSpan};
    pub use ratatui::widgets::{
        Block, BorderType, Borders, Clear, LineGauge, Padding, Paragraph, ScrollbarState, Wrap,
    };
    pub use ratatui_textarea::TextArea;
    pub use tui_widget_list::{ListBuilder, ListState, ListView};
}
