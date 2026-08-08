use crossterm::event::Event;
use ratatui::prelude::*;
use ratatui::text::ToSpan;
use ratatui::widgets::{Block, BorderType, Borders, Padding, StatefulWidget, Widget};
use tui_widget_list::{ListBuilder, ListState, ListView};

use crate::types::{Message, Node};
use crate::ui::helpers::{ListStateExt, default_scrollbar, hops_to_spans, routing_error_to_span, short_name_to_span};
use crate::ui::widget::PlaceholderWidget;

pub struct ReactionViewerState {
    list_state: ListState,
}

impl ReactionViewerState {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
        }
    }

    pub fn handle_event(&mut self, event: &Event, reactions_count: usize) -> anyhow::Result<bool> {
        if self.list_state.handle_navigation_events(event, Some(reactions_count)) {
            return Ok(true);
        }

        Ok(false)
    }
}

pub struct ReactionViewerItem<'a> {
    pub reaction: &'a Message,
    pub node: &'a Node,
    pub is_my_node: bool,
}

pub struct ReactionViewerWidget<'a> {
    reactions: Vec<ReactionViewerItem<'a>>,
}

impl<'a> ReactionViewerWidget<'a> {
    pub fn new(reactions: Vec<ReactionViewerItem<'a>>) -> Self {
        Self { reactions }
    }
}

impl<'a> StatefulWidget for ReactionViewerWidget<'a> {
    type State = ReactionViewerState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = Block::bordered()
            .border_type(BorderType::Thick)
            .padding(Padding::new(0, 1, 1, 0))
            .title(format!(" reactions: {} ", self.reactions.len()));

        let block_area = block.inner(area);
        block.render(area, buf);

        // placeholder if no reactions
        if self.reactions.is_empty() {
            PlaceholderWidget::dark_gray("no reactions").render(block_area, buf);
            return;
        }

        // list
        if let Some(selected) = state.list_state.selected
            && selected > self.reactions.len() - 1
        {
            state.list_state.select(None);
        }

        if state.list_state.selected.is_none() && !self.reactions.is_empty() {
            state.list_state.select(Some(0));
        }

        const ITEM_HEIGHT: usize = 3;
        let is_scrollable = self.reactions.len() * ITEM_HEIGHT - 1 > block_area.height as usize;

        let list_builder = ListBuilder::new(|context| {
            let reaction = &self.reactions[context.index];

            let item = ReactionWidget {
                item: reaction,
                is_selected: context.is_selected,
                is_scrollable,
            };

            (
                item,
                if context.index < self.reactions.len() - 1 {
                    ITEM_HEIGHT as u16
                } else {
                    ITEM_HEIGHT as u16 - 1
                },
            )
        });

        let list = ListView::new(list_builder, self.reactions.len())
            .scrollbar(default_scrollbar())
            .infinite_scrolling(false);

        list.render(block_area, buf, &mut state.list_state);
    }
}

struct ReactionWidget<'a> {
    pub item: &'a ReactionViewerItem<'a>,
    pub is_selected: bool,
    pub is_scrollable: bool,
}

impl<'a> Widget for ReactionWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let area = Rect::new(area.x, area.y, area.width, 2);

        let block = Block::new()
            .padding(Padding::new(1, if self.is_scrollable { 3 } else { 0 }, 0, 0))
            .borders(Borders::LEFT)
            .border_type(BorderType::QuadrantInside)
            .border_style(Style::new().fg(if self.is_selected { Color::Yellow } else { Color::Black }));

        let block_area = block.inner(area);
        block.render(area, buf);

        let v = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(block_area);
        let v0_h = Layout::horizontal([Constraint::Fill(1), Constraint::Length(1), Constraint::Length(2)]).split(v[0]);
        let v1_h = Layout::horizontal([Constraint::Fill(1), Constraint::Length(1), Constraint::Length(5)]).split(v[1]);

        // first line
        Line::from(vec![
            short_name_to_span(self.item.node, self.item.is_my_node),
            " ".to_span(),
            self.item.node.long_name().to_span(),
        ])
        .render(v0_h[0], buf);

        Span::from(&self.item.reaction.text).render(v0_h[2], buf);

        // second line
        Line::from(if self.item.is_my_node {
            vec![routing_error_to_span(self.item.reaction.routing_error)]
        } else {
            hops_to_spans(self.item.reaction, self.item.is_my_node)
        })
        .render(v1_h[0], buf);

        Span::from(
            self.item
                .reaction
                .datetime
                .with_timezone(&chrono::Local)
                .format("%H:%M")
                .to_string(),
        )
        .dark_gray()
        .render(v1_h[2], buf);
    }
}
