use std::marker::PhantomData;

use crossterm::event::Event;
use emoji::Emoji;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Padding, StatefulWidget, Widget},
};
use ratatui_textarea::TextArea;
use tui_widget_list::{ListBuilder, ListState, ListView};

use crate::ui::helpers::{ListStateExt, default_scrollbar};

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
            emojis: emoji::search::search_name(""),
        }
    }

    pub fn get_value(&self) -> Option<&'static Emoji> {
        self.list_state.selected.and_then(|i| self.emojis.get(i).cloned())
    }

    pub fn reset(&mut self) {
        self.input_widget.clear();
        self.emojis = emoji::search::search_name("");
        self.list_state.select(Some(0));
    }

    pub fn handle_event(&mut self, event: Event) -> anyhow::Result<bool> {
        if self
            .list_state
            .handle_navigation_events(&event, Some(self.emojis.len()))
        {
            return Ok(true);
        }

        self.input_widget.input(event);
        self.emojis = emoji::search::search_name(&self.input_widget.lines()[0]);

        if !self.emojis.is_empty() {
            self.list_state.select(Some(0));
        }

        Ok(true)
    }
}

pub struct EmojiSelectorWidget<'a> {
    _marker: PhantomData<&'a ()>,
}

impl<'a> EmojiSelectorWidget<'a> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<'a> StatefulWidget for EmojiSelectorWidget<'a> {
    type State = EmojiSelectorState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = Block::bordered()
            .border_type(BorderType::Thick)
            .padding(Padding::symmetric(1, 0))
            .title(" emoji selector ");

        let block_area = block.inner(area);
        block.render(area, buf);

        let v = Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).split(block_area);

        // input
        let input_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().dark_gray())
            .padding(Padding::symmetric(1, 0));

        let input_block_area = input_block.inner(v[0]);
        input_block.render(v[0], buf);

        state.input_widget.render(input_block_area, buf);

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
            .infinite_scrolling(false);

        list.render(v[1], buf, &mut state.list_state);
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
