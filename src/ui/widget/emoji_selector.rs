use std::marker::PhantomData;

use crossterm::event::{Event, KeyCode, KeyEvent};
use emojis::Emoji;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Padding, StatefulWidget, Widget},
};
use ratatui_textarea::TextArea;
use tui_widget_list::{ListBuilder, ListState, ListView};

use crate::ui::helpers::{ListStateExt, TextAreaExt, default_scrollbar};

const MAX_EMOJIS_LIMIT: usize = 100;

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
            emojis: emojis::iter().take(MAX_EMOJIS_LIMIT).collect(),
        }
    }

    pub fn is_selected(&self) -> bool {
        !self.emojis.is_empty() && self.list_state.selected.is_some()
    }

    pub fn reset(&mut self) {
        self.input_widget.clear();
        self.emojis = emojis::iter().take(MAX_EMOJIS_LIMIT).collect();
        self.list_state.select(Some(0));
    }

    pub fn handle_event(&mut self, event: Event, emit: &mut impl FnMut(EmojiSelectorWidgetEvent)) -> bool {
        self.list_state.fix_selection(self.emojis.len());

        if self
            .list_state
            .handle_navigation_events(&event, Some(self.emojis.len()))
        {
            return true;
        }

        match event {
            Event::Key(KeyEvent { code, modifiers, .. }) => match code {
                KeyCode::Enter if modifiers.is_empty() => {
                    if let Some(emoji) = self.get_value() {
                        emit(EmojiSelectorWidgetEvent::EmojiSubmitted(emoji));
                        self.reset();
                        return true;
                    }
                }
                KeyCode::Esc if modifiers.is_empty() => {
                    emit(EmojiSelectorWidgetEvent::CloseRequested);
                    self.reset();
                    return true;
                }
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Esc | KeyCode::F(12) => {
                    // capture these events to prevent handling by input widget
                    return false;
                }
                _ => {
                    self.input_widget.input(event);
                    self.update();
                    return true;
                }
            },
            Event::Paste(text) => {
                self.input_widget.insert_as_lines(&text);
                self.update();
            }
            _ => {
                self.input_widget.input(event);
                self.update();
                return true;
            }
        }

        true
    }

    fn get_value(&self) -> Option<&'static Emoji> {
        self.list_state.selected.and_then(|i| self.emojis.get(i).cloned())
    }

    fn update(&mut self) {
        self.emojis = emojis::iter()
            .filter(|e| {
                e.name()
                    .to_lowercase()
                    .contains(&self.input_widget.lines()[0].to_lowercase())
            })
            .take(MAX_EMOJIS_LIMIT)
            .collect();

        if !self.emojis.is_empty() {
            self.list_state.select(Some(0));
        }
    }
}

pub struct EmojiSelectorWidget<'a> {
    _marker: PhantomData<&'a ()>,
}

impl<'a> EmojiSelectorWidget<'a> {
    pub fn new() -> Self {
        Self { _marker: PhantomData }
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

        state.list_state.fix_selection(state.emojis.len());

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
            Span::from(self.emoji.as_str()),
            Span::from("  "),
            Span::from(self.emoji.name()),
        ])
        .add_modifier(if self.is_selected {
            Modifier::REVERSED
        } else {
            Modifier::empty()
        });

        item.render(block_area, buf);
    }
}

pub enum EmojiSelectorWidgetEvent<'a> {
    EmojiSubmitted(&'a Emoji),
    CloseRequested,
}
