use chrono::Local;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
use ratatui::{
    layout::*,
    prelude::*,
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Widget, Wrap},
};
use tracing_unwrap::OptionExt;
use tui_scrollview::{ScrollView, ScrollViewState, ScrollbarVisibility};
use tui_widget_list::{ListBuilder, ListState, ListView};
use uuid::Uuid;

use crate::{
    state::State,
    types::{Message, Node, UNKNOWN_NODE},
    ui::helpers::{
        DEFAULT_SCROLLBAR_SET, StringExt, hops_to_spans, humanize_datetime, routing_error_to_span, short_name_to_span,
    },
};

#[derive(Default)]
pub struct MessageInfoWidgetState {
    scroll_view_state: ScrollViewState,
    reactions_list_state: ListState,
}

impl MessageInfoWidgetState {
    pub fn handle_event(&mut self, event: Event, emit: &mut impl FnMut(MessageInfoWidgetEvent)) -> bool {
        match event {
            Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                modifiers,
                ..
            }) => match code {
                KeyCode::Up if modifiers.is_empty() => {
                    self.scroll_view_state.scroll_up();
                    return true;
                }
                KeyCode::Down if modifiers.is_empty() => {
                    self.scroll_view_state.scroll_down();
                    return true;
                }
                KeyCode::Esc if modifiers.is_empty() => {
                    self.scroll_view_state.scroll_to_top();
                    emit(MessageInfoWidgetEvent::CloseRequested);
                    return true;
                }
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Esc | KeyCode::F(12) => {
                    // capture these events to prevent handling
                    return false;
                }
                _ => {}
            },
            Event::Mouse(MouseEvent { kind, .. }) => match kind {
                MouseEventKind::ScrollUp => {
                    self.scroll_view_state.scroll_up();
                    return true;
                }
                MouseEventKind::ScrollDown => {
                    self.scroll_view_state.scroll_down();
                    return true;
                }
                _ => {}
            },
            _ => {}
        }

        true
    }
}

pub struct MessageInfoWidget<'a> {
    data: MessageInfoWidgetData<'a>,
}

impl<'a> MessageInfoWidget<'a> {
    pub fn new(data: MessageInfoWidgetData<'a>) -> Self {
        Self { data }
    }
}

impl<'a> StatefulWidget for MessageInfoWidget<'a> {
    type State = MessageInfoWidgetState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = Block::bordered()
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .padding(Padding::new(2, 2, 1, 0))
            .title(" expanded view ");

        let block_area = block.inner(area);
        block.render(area, buf);

        let replying_message_paragraph = self.data.replying_message.map(|m| {
            Paragraph::new(Line::from(vec![Span::from("“"), Span::from(m.text_oneline()), Span::from("”")]).magenta())
                .wrap(Wrap { trim: false })
        });

        let replying_message_paragraph_height = replying_message_paragraph
            .as_ref()
            .map(|p| p.line_count(block_area.width - 2) as u16)
            .unwrap_or(0);

        let message_paragraph =
            Paragraph::new(self.data.message.text.to_hyperlinked_lines()).wrap(Wrap { trim: false });

        let message_paragraph_height = message_paragraph.as_ref().line_count(block_area.width - 2) as u16;

        let has_reactions = !self.data.reactions.is_empty();

        let content_size = Size::new(
            block_area.width - 2,
            [
                if self.data.replying_node.is_some() {
                    1 + replying_message_paragraph_height + 1 // header + replying message + gap
                } else {
                    0
                },
                1,                        // message header
                message_paragraph_height, // message
                if has_reactions {
                    1 + 1 + self.data.reactions.len() as u16 // gap + header + reactions
                } else {
                    0
                },
                1, // gap
                1, // line
                1, // meta header
                1, // meta
            ]
            .iter()
            .sum(),
        );

        let mut scroll_view = ScrollView::new(content_size)
            .horizontal_scrollbar_visibility(ScrollbarVisibility::Never)
            .vertical_scrollbar_symbols(DEFAULT_SCROLLBAR_SET)
            .vertical_scrollbar_style(Style::new().dark_gray());

        let [
            replying_header_area,
            replying_message_area,
            _,
            message_header_area,
            message_area,
            _,
            reactions_header_area,
            reactions_area,
            _,
            line_area,
            meta_uuid_area,
            meta_packet_id_area,
        ] = Layout::vertical([
            Constraint::Length(if replying_message_paragraph_height > 0 { 1 } else { 0 }), // replying message header
            Constraint::Length(replying_message_paragraph_height),                         // replying message
            Constraint::Length(if replying_message_paragraph_height > 0 { 1 } else { 0 }), // gap
            Constraint::Length(1),                                                         // message header
            Constraint::Length(message_paragraph_height),                                  // message
            Constraint::Length(if has_reactions { 1 } else { 0 }),                         // gap
            Constraint::Length(if has_reactions { 1 } else { 0 }),                         // reactions header
            Constraint::Length(self.data.reactions.len() as u16),                          // reactions
            Constraint::Length(1),                                                         // gap
            Constraint::Length(1),                                                         // line
            Constraint::Length(1),                                                         // meta UUID
            Constraint::Length(1),                                                         // meta packet ID
        ])
        .areas(Rect::new(0, 0, content_size.width, content_size.height));

        // replying node
        if let Some(replying_node) = self.data.replying_node {
            let replying_header_area_h =
                Layout::horizontal([Constraint::Fill(3), Constraint::Fill(1)]).split(replying_header_area);

            scroll_view.render_widget(
                Line::from(vec![
                    Span::from("IN REPLY TO ").magenta(),
                    short_name_to_span(replying_node, false).on_magenta(),
                    Span::from(format!(" {}", replying_node.long_name())),
                ]),
                replying_header_area_h[0],
            );

            if let Some(replying_message) = self.data.replying_message {
                scroll_view.render_widget(
                    Line::from(
                        Span::from(humanize_datetime(replying_message.datetime.with_timezone(&Local))).dark_gray(),
                    )
                    .right_aligned(),
                    replying_header_area_h[1],
                );
            }
        }

        // replying message
        if self.data.replying_message.is_some() {
            scroll_view.render_widget(replying_message_paragraph, replying_message_area);
        }

        // message
        let message_header_area_h =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(10), Constraint::Length(17)])
                .split(message_header_area);

        scroll_view.render_widget(
            Line::from(vec![
                Span::from("FROM ").magenta(),
                short_name_to_span(self.data.node, self.data.is_my_node),
                Span::from(format!(" {}", self.data.node.long_name())),
            ]),
            message_header_area_h[0],
        );

        scroll_view.render_widget(
            Line::from(if self.data.is_my_node {
                vec![routing_error_to_span(self.data.message.routing_error)]
            } else {
                hops_to_spans(self.data.message, self.data.is_my_node)
            }),
            message_header_area_h[1],
        );

        scroll_view.render_widget(
            Line::from(Span::from(humanize_datetime(self.data.message.datetime.with_timezone(&Local))).dark_gray())
                .right_aligned(),
            message_header_area_h[2],
        );

        scroll_view.render_widget(message_paragraph, message_area);

        // reactions
        if has_reactions {
            scroll_view.render_widget(Span::from("REACTIONS ").magenta(), reactions_header_area);

            let reactions_list_builder = ListBuilder::new(|context| {
                let item = ReactionWidget {
                    item: self.data.reactions.get(context.index).unwrap(),
                };

                (item, 1)
            });

            let reactions_list = ListView::new(reactions_list_builder, self.data.reactions.len());

            scroll_view.render_stateful_widget(reactions_list, reactions_area, &mut state.reactions_list_state);
        }

        // meta
        scroll_view.render_widget(
            Span::from("─".repeat(line_area.width as usize)).dark_gray().dim(),
            line_area,
        );

        let meta_uuid_area_h = Layout::horizontal([Constraint::Length(12), Constraint::Fill(1)]).split(meta_uuid_area);
        let meta_packet_id_area_h =
            Layout::horizontal([Constraint::Length(12), Constraint::Fill(1)]).split(meta_packet_id_area);

        scroll_view.render_widget(Span::from("UUID").cyan(), meta_uuid_area_h[0]);
        scroll_view.render_widget(
            Span::from(self.data.message.id.to_string()).dark_gray(),
            meta_uuid_area_h[1],
        );

        scroll_view.render_widget(Span::from("PACKET ID").cyan(), meta_packet_id_area_h[0]);
        scroll_view.render_widget(
            Span::from(self.data.message.packet_id.to_string()).dark_gray(),
            meta_packet_id_area_h[1],
        );

        // final render
        scroll_view.render(block_area, buf, &mut state.scroll_view_state);
    }
}

pub struct MessageInfoWidgetData<'a> {
    node: &'a Node,
    message: &'a Message,
    reactions: Vec<ReactionItem<'a>>,
    replying_node: Option<&'a Node>,
    replying_message: Option<&'a Message>,
    is_my_node: bool,
}

impl<'a> MessageInfoWidgetData<'a> {
    pub fn new(message_id: Uuid, state: &'a State) -> Self {
        let message = state.messages.get(&message_id).expect_or_log("message should exist");

        let reactions: Vec<ReactionItem> = message
            .reactions
            .iter()
            .filter_map(|message_id| {
                let Some(reaction) = state.messages.get(message_id) else {
                    return None;
                };

                let node = state.nodes.get(&reaction.from).unwrap_or(&UNKNOWN_NODE);

                Some(ReactionItem {
                    message: reaction,
                    node,
                    is_my_node: state.is_my_node(node.key),
                })
            })
            .collect();

        let replying_node = if !message.reply_id.is_nil() {
            state
                .messages
                .get(&message.reply_id)
                .map(|m| state.nodes.get(&m.from).unwrap_or(&UNKNOWN_NODE))
        } else {
            None
        };

        let replying_message = if !message.reply_id.is_nil() {
            state.messages.get(&message.reply_id)
        } else {
            None
        };

        let node = state.nodes.get(&message.from).unwrap_or(&UNKNOWN_NODE);

        Self {
            node,
            message,
            reactions,
            replying_node,
            replying_message,
            is_my_node: state.is_my_node(node.key),
        }
    }
}

struct ReactionWidget<'a> {
    pub item: &'a ReactionItem<'a>,
}

impl<'a> Widget for ReactionWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let [emoji_area, node_area, hops_area, date_area] = Layout::horizontal([
            Constraint::Length(5),
            Constraint::Fill(1),
            Constraint::Length(10),
            Constraint::Length(17),
        ])
        .areas(area);

        // emoji
        Line::from(Span::from(&self.item.message.text)).render(emoji_area, buf);

        // node
        Line::from(vec![
            short_name_to_span(self.item.node, self.item.is_my_node),
            Span::from(" "),
            Span::from(self.item.node.long_name()),
        ])
        .render(node_area, buf);

        // hops, routing error
        Line::from(if self.item.is_my_node {
            vec![routing_error_to_span(self.item.message.routing_error)]
        } else {
            hops_to_spans(self.item.message, self.item.is_my_node)
        })
        .render(hops_area, buf);

        // datetime
        Line::from(humanize_datetime(
            self.item.message.datetime.with_timezone(&chrono::Local),
        ))
        .right_aligned()
        .dark_gray()
        .render(date_area, buf);
    }
}

pub enum MessageInfoWidgetEvent {
    CloseRequested,
}

struct ReactionItem<'a> {
    pub message: &'a Message,
    pub node: &'a Node,
    pub is_my_node: bool,
}
