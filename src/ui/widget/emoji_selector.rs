use std::marker::PhantomData;

use crossterm::event::{Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use emoji::Emoji;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Padding, StatefulWidget, Widget},
};
use ratatui_textarea::TextArea;
use tui_widget_list::{ListBuilder, ListState, ListView};

use crate::{
    types::Hotkey,
    ui::{helpers::default_scrollbar, widget::HotkeysWidget},
};

pub struct EmojiSelectorState<'a> {
    input_widget: TextArea<'a>,
    list_state: ListState,
    emojis: Vec<&'static Emoji>,
}

impl<'a> EmojiSelectorState<'a> {
    pub fn new() -> Self {
        let mut input_widget = TextArea::default();
        input_widget.set_placeholder_text("start typing emoji name...");
        input_widget.set_cursor_line_style(Style::default());

        Self {
            input_widget,
            list_state: ListState::default(),
            emojis: emoji::search::search_tts("", "en"),
        }
    }

    pub fn get_value(&self) -> Option<&'static Emoji> {
        self.list_state
            .selected
            .and_then(|i| self.emojis.get(i).cloned())
    }

    pub fn reset(&mut self) {
        self.input_widget.clear();
        self.emojis = emoji::search::search_tts("", "en");
        self.list_state.select(Some(0));
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Up => {
                    self.list_state.previous();
                }
                KeyCode::Down => {
                    self.list_state.next();
                }
                _ => {
                    self.input_widget.input(event);
                    self.emojis = emoji::search::search_tts(&self.input_widget.lines()[0], "en");

                    if !self.emojis.is_empty() {
                        self.list_state.select(Some(0));
                    }
                }
            },
            Event::Mouse(MouseEvent { kind, .. }) => match kind {
                MouseEventKind::ScrollUp => {
                    self.list_state.previous();
                }
                MouseEventKind::ScrollDown => {
                    self.list_state.next();
                }
                _ => {}
            },
            _ => {}
        }
    }
}

pub struct EmojiSelectorWidget<'a> {
    _marker: PhantomData<&'a ()>,
}

impl<'a> EmojiSelectorWidget<'a> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData::default(),
        }
    }
}

impl<'a> StatefulWidget for EmojiSelectorWidget<'a> {
    type State = EmojiSelectorState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .padding(Padding::symmetric(1, 0));

        let block_area = block.inner(area);
        block.render(area, buf);

        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(1),
            ])
            .split(block_area);

        // input
        state.input_widget.render(v[0], buf);

        // list
        let list_builder = ListBuilder::new(|context| {
            let emoji = &state.emojis[context.index];

            let item = EmojiWidget {
                emoji,
                is_selected: context.is_selected,
            };

            (item, 1)
        });

        let list = ListView::new(list_builder, state.emojis.len())
            .scrollbar(default_scrollbar())
            .infinite_scrolling(false)
            .block(Block::new().padding(Padding::symmetric(0, 1)));

        list.render(v[1], buf, &mut state.list_state);

        // hotkeys
        HotkeysWidget::new(&vec![
            Hotkey::new("enter", "insert"),
            Hotkey::new("esc", "close"),
        ])
        .render(v[2], buf);
    }
}

struct EmojiWidget {
    pub emoji: &'static Emoji,
    pub is_selected: bool,
}

impl Widget for EmojiWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::new().padding(Padding::right(2));
        let block_area = block.inner(area);

        block.render(area, buf);

        let item = Line::from(vec![
            Span::from(self.emoji.glyph),
            Span::from("  "),
            Span::from(self.emoji.name),
        ])
        .add_modifier(if self.is_selected {
            Modifier::REVERSED
        } else {
            Modifier::empty()
        });

        item.render(block_area, buf);
    }
}
