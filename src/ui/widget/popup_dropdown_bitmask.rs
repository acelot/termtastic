use std::marker::PhantomData;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Clear, Padding, StatefulWidget, Widget},
};
use tui_widget_list::{ListBuilder, ListState, ListView};

use crate::ui::helpers::ListStateExt;
use crate::{types::FormBitMaskVariant, ui::helpers::default_scrollbar};

const MAX_VISIBLE_DROPDOWN_ITEMS: usize = 8;

pub struct PopupDropdownBitmaskState<'a> {
    title: &'a str,
    variants: &'a Vec<FormBitMaskVariant>,
    selected: u32,
    list_state: ListState,
}

impl<'a> PopupDropdownBitmaskState<'a> {
    pub fn new(title: &'a str, variants: &'a Vec<FormBitMaskVariant>, selected: u32) -> Self {
        Self {
            title,
            variants,
            selected,
            list_state: ListState::default(),
        }
    }

    pub fn get_value(&self) -> u32 {
        self.selected
    }

    pub fn handle_event(&mut self, event: Event) -> anyhow::Result<bool> {
        if self
            .list_state
            .handle_navigation_events(&event, Some(self.variants.len()))
        {
            return Ok(true);
        }

        match event {
            Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                modifiers,
                ..
            }) if modifiers.is_empty() => match code {
                KeyCode::Char(' ') if let Some(index) = self.list_state.selected => {
                    let variant = self.variants.get(index).unwrap();
                    let is_checked = self.selected & variant.value > 0;

                    if is_checked {
                        self.selected &= !variant.value;
                    } else {
                        self.selected |= variant.value;
                    }

                    return Ok(true);
                }
                _ => {}
            },
            _ => {}
        }

        Ok(false)
    }
}

pub struct PopupDropdownBitmaskWidget<'a> {
    width: u16,
    _marker: PhantomData<&'a ()>,
}

impl<'a> PopupDropdownBitmaskWidget<'a> {
    pub fn new(width: u16) -> Self {
        Self {
            width,
            _marker: PhantomData,
        }
    }
}

impl<'a> StatefulWidget for PopupDropdownBitmaskWidget<'a> {
    type State = PopupDropdownBitmaskState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let height = state.variants.len().min(MAX_VISIBLE_DROPDOWN_ITEMS) as u16 + 2;
        let popup_area = area.centered(Constraint::Length(self.width), Constraint::Length(height));

        let popup_block = Block::bordered()
            .border_type(BorderType::Thick)
            .padding(Padding::symmetric(1, 0))
            .title(format!(" {} ", state.title.trim()));

        let popup_block_area = popup_block.inner(popup_area);

        Clear.render(popup_area, buf);
        popup_block.render(popup_area, buf);

        if state.list_state.selected.is_none() && !state.variants.is_empty() {
            state.list_state.select(Some(0));
        }

        let list_builder = ListBuilder::new(|context| {
            let variant = state.variants.get(context.index).unwrap();
            let is_checked = state.selected & variant.value > 0;

            let item = Line::from(vec![
                Span::from(if is_checked { "[■]" } else { "[_]" }),
                Span::from(" "),
                Span::from(&variant.title),
            ])
            .patch_style(if context.is_selected {
                Style::new().black().on_yellow()
            } else {
                Style::new()
            });

            (item, 1)
        });

        let list = ListView::new(list_builder, state.variants.len())
            .infinite_scrolling(false)
            .scrollbar(default_scrollbar());

        list.render(popup_block_area, buf, &mut state.list_state);
    }
}
