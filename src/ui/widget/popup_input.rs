use std::marker::PhantomData;

use crossterm::event::Event;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Padding, StatefulWidget, Widget},
};
use ratatui_textarea::TextArea;

pub struct PopupInputState<'a> {
    title: &'a str,
    textarea: TextArea<'a>,
    error: Option<String>,
}

impl<'a> PopupInputState<'a> {
    pub fn new(title: &'a str, placeholder: Option<&'a str>, value: String) -> Self {
        let mut textarea = TextArea::new(vec![value]);
        textarea.set_cursor_line_style(Style::default());
        textarea.select_all();

        if let Some(text) = placeholder {
            textarea.set_placeholder_text(text);
        }

        Self {
            title,
            textarea,
            error: None,
        }
    }

    pub fn set_error<S: Into<String>>(&mut self, text: S) {
        self.error = Some(text.into());
    }

    pub fn get_value(&self) -> String {
        self.textarea.lines()[0].clone()
    }

    pub fn handle_event(&mut self, event: Event) {
        if self.textarea.input(event) {
            self.error = None;
        }
    }
}

pub struct PopupInputWidget<'a> {
    width: u16,
    _marker: PhantomData<&'a ()>,
}

impl<'a> PopupInputWidget<'a> {
    pub fn new(width: u16) -> Self {
        Self {
            width,
            _marker: PhantomData::default(),
        }
    }
}

impl<'a> StatefulWidget for PopupInputWidget<'a> {
    type State = PopupInputState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let popup_area = Rect {
            x: area.x + area.width / 2 - self.width / 2,
            y: area.y + area.height / 2 - 2,
            width: self.width,
            height: 4,
        };

        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(1)])
            .split(popup_area);

        let color = if state.error.is_some() {
            Color::Red
        } else {
            Color::Reset
        };

        let textarea_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(color))
            .padding(Padding::symmetric(1, 0))
            .title(format!(" {} ", state.title));

        let textarea_block_area = textarea_block.inner(v[0]);

        Clear.render(popup_area, buf);
        textarea_block.render(v[0], buf);

        state.textarea.set_style(Style::new().fg(color));
        state.textarea.render(textarea_block_area, buf);

        if let Some(text) = &state.error {
            Line::from(Span::from(text).red()).render(v[1], buf);
        }
    }
}
