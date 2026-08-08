use std::marker::PhantomData;

use crossterm::event::Event;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Clear, Padding, StatefulWidget, Widget},
};
use ratatui_textarea::TextArea;

pub struct PopupInputState<'a> {
    title: Option<&'a str>,
    textarea: TextArea<'a>,
    error: Option<String>,
}

impl Default for PopupInputState<'_> {
    fn default() -> Self {
        Self {
            title: None,
            textarea: TextArea::new(vec![]),
            error: None,
        }
    }
}

impl<'a> PopupInputState<'a> {
    pub fn new<S: Into<String>>(title: Option<&'a str>, placeholder: Option<&'a str>, value: S) -> Self {
        let mut textarea = TextArea::new(vec![value.into()]);
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

    pub fn get_value(&self) -> String {
        self.textarea.lines().first().unwrap_or(&String::new()).to_owned()
    }

    pub fn set_title(&mut self, title: Option<&'a str>) {
        self.title = title;
    }

    pub fn set_placeholder<S: Into<String>>(&mut self, placeholder: S) {
        self.textarea.set_placeholder_text(placeholder);
    }

    pub fn set_error<S: Into<String>>(&mut self, text: S) {
        self.error = Some(text.into());
    }

    pub fn insert_str(&mut self, text: &str) {
        self.textarea.insert_str(text);
    }

    pub fn reset(&mut self) {
        self.error = None;
        self.textarea.clear();
    }

    pub fn handle_event(&mut self, event: Event) -> anyhow::Result<bool> {
        if self.textarea.input(event) {
            self.error = None;
        }

        Ok(true)
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
            _marker: PhantomData,
        }
    }
}

impl<'a> StatefulWidget for PopupInputWidget<'a> {
    type State = PopupInputState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let popup_area = area.centered(Constraint::Length(self.width), Constraint::Length(4));

        let v = Layout::vertical([Constraint::Length(3), Constraint::Length(1)]).split(popup_area);

        let color = if state.error.is_some() {
            Color::Red
        } else {
            Color::Reset
        };

        let mut textarea_block = Block::bordered()
            .border_type(BorderType::Thick)
            .border_style(Style::new().fg(color))
            .padding(Padding::symmetric(1, 0));

        if let Some(title) = state.title {
            textarea_block = textarea_block.title(format!(" {} ", title.trim()));
        }

        let textarea_block_area = textarea_block.inner(v[0]);

        Clear.render(popup_area, buf);
        textarea_block.render(v[0], buf);

        state.textarea.set_style(Style::new().fg(color));
        state.textarea.set_cursor_line_style(Style::default());
        state.textarea.render(textarea_block_area, buf);

        if let Some(text) = &state.error {
            Line::from(Span::from(text).red()).render(v[1], buf);
        }
    }
}
