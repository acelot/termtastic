use itertools::Itertools;
use ordermap::OrderMap;
use ratatui::{
    layout::*,
    prelude::*,
    text::{Line, Span, ToSpan},
    widgets::{Block, Borders, Padding, Paragraph, Widget, Wrap},
};
use tracing_unwrap::OptionExt;
use uuid::Uuid;

use crate::{
    state::State,
    types::{Message, Node, UNKNOWN_NODE},
    ui::helpers::{StringExt, hops_to_spans, humanize_datetime, routing_error_to_span, short_name_to_span},
};

const REACTIONS_LINE_MAX_WIDTH: usize = 20;
const MESSAGE_MAX_HEIGHT: u16 = 6;

pub struct MessageWidget<'a> {
    data: MessageWidgetData<'a>,
    message_paragraph: Paragraph<'a>,
    reactions_line: Line<'a>,
}

impl<'a> MessageWidget<'a> {
    pub fn new(data: MessageWidgetData<'a>) -> Self {
        let message_paragraph = get_text_paragraph(data.message, data.replying_message);
        let reactions_line = get_reactions_line(data.reactions.clone());

        Self {
            data,
            message_paragraph,
            reactions_line,
        }
    }

    pub fn height(&self, width: u16) -> (u16, bool) {
        let text_height = self.message_paragraph.line_count(
            width
                .saturating_sub(self.reactions_line.width().min(REACTIONS_LINE_MAX_WIDTH) as u16)
                .saturating_sub(4),
        ) as u16;

        let is_overheight = text_height > MESSAGE_MAX_HEIGHT;

        if is_overheight {
            (1 + MESSAGE_MAX_HEIGHT + 1, true)
        } else {
            (1 + text_height, false)
        }
    }
}

impl<'a> Widget for MessageWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let (height, is_overheight) = self.height(area.width);

        let area = Rect {
            x: area.x,
            y: area.y,
            width: area.width - 2,
            height,
        };

        let block = Block::bordered()
            .borders(Borders::LEFT)
            .border_set(if self.data.is_selected {
                symbols::border::THICK
            } else {
                symbols::border::PLAIN
            })
            .border_style(Style::new().fg(if self.data.is_highlighted {
                Color::Cyan
            } else if self.data.is_selected {
                Color::Yellow
            } else {
                Color::DarkGray
            }))
            .padding(Padding::symmetric(1, 0));

        let block_area = block.inner(area);
        block.render(area, buf);

        let v = Layout::vertical(
            [
                Some(Constraint::Length(1)),
                Some(Constraint::Fill(1)),
                is_overheight.then_some(Constraint::Length(1)),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<Constraint>>(),
        )
        .split(block_area);

        let v0_h = Layout::horizontal([Constraint::Fill(4), Constraint::Fill(2), Constraint::Fill(1)])
            .flex(Flex::SpaceBetween)
            .split(v[0]);

        let v1_h = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(2),
            Constraint::Length(self.reactions_line.width().min(REACTIONS_LINE_MAX_WIDTH) as u16),
        ])
        .split(v[1]);

        // first line
        if let Some(rep_node) = self.data.replying_node {
            Line::from(vec![
                short_name_to_span(self.data.node, self.data.is_my_node),
                " → ".to_span().dark_gray(),
                short_name_to_span(rep_node, self.data.is_my_node).on_magenta(),
            ])
            .render(v0_h[0], buf);
        } else {
            Line::from(vec![
                short_name_to_span(self.data.node, self.data.is_my_node),
                " ".to_span(),
                self.data.node.long_name().to_span().bold(),
            ])
            .render(v0_h[0], buf);
        }

        if !self.data.is_my_node {
            Line::from(hops_to_spans(self.data.message, false)).render(v0_h[1], buf);
        } else {
            routing_error_to_span(self.data.message.routing_error).render(v0_h[1], buf);
        }

        Line::from(
            Span::from(humanize_datetime(
                self.data.message.datetime.with_timezone(&chrono::Local),
            ))
            .dark_gray(),
        )
        .right_aligned()
        .render(v0_h[2], buf);

        // second line
        self.message_paragraph.render(v1_h[0], buf);
        self.reactions_line.render(v1_h[2], buf);

        // thirs line
        if is_overheight {
            Line::from(
                [
                    Some(Span::from("(trimmed)").red().add_modifier(if self.data.is_selected {
                        Modifier::empty()
                    } else {
                        Modifier::DIM
                    })),
                    self.data.is_selected.then_some(Span::from(" press ").dark_gray().dim()),
                    self.data.is_selected.then_some(Span::from("<enter>").dim()),
                    self.data
                        .is_selected
                        .then_some(Span::from(" to expand").dark_gray().dim()),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<Span<'_>>>(),
            )
            .render(v[2], buf);
        }
    }
}

pub struct MessageWidgetData<'a> {
    node: &'a Node,
    message: &'a Message,
    reactions: Vec<&'a Message>,
    replying_node: Option<&'a Node>,
    replying_message: Option<&'a Message>,
    is_my_node: bool,
    is_selected: bool,
    is_highlighted: bool,
}

impl<'a> MessageWidgetData<'a> {
    pub fn new(message_id: Uuid, state: &'a State) -> Self {
        let message = state.messages.get(&message_id).expect_or_log("message should exist");

        let reactions: Vec<&Message> = message
            .reactions
            .iter()
            .filter_map(|message_id| state.messages.get(message_id))
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
            is_selected: false,
            is_highlighted: false,
        }
    }

    pub fn selected(mut self, value: bool) -> Self {
        self.is_selected = value;
        self
    }

    pub fn highlighted(mut self, value: bool) -> Self {
        self.is_highlighted = value;
        self
    }
}

fn get_text_paragraph<'a>(message: &'a Message, replied_message: Option<&'a Message>) -> Paragraph<'a> {
    let reply_line = replied_message
        .map(|msg| Line::from(vec!["“".to_span(), Span::from(msg.text_oneline()), "”".to_span()]).magenta());

    let text_lines: Vec<Line<'_>> = message.text.to_hyperlinked_lines();

    Paragraph::new(reply_line.into_iter().chain(text_lines).collect::<Vec<Line<'_>>>()).wrap(Wrap { trim: false })
}

fn get_reactions_line<'a>(reactions: Vec<&'a Message>) -> Line<'a> {
    let summary = reactions
        .iter()
        .sorted_by_key(|r| r.datetime)
        .fold(OrderMap::new(), |mut acc, r| {
            *acc.entry(&r.text).or_insert(0) += 1;
            acc
        });

    #[allow(unstable_name_collisions)]
    Line::from(
        summary
            .into_iter()
            .map(|(emoji, nodes_count)| {
                if nodes_count > 1 {
                    Span::from(format!("{}:{}", emoji, nodes_count)).dark_gray()
                } else {
                    emoji.to_span()
                }
            })
            .intersperse(" ".to_span())
            .collect::<Vec<Span>>(),
    )
    .right_aligned()
}
