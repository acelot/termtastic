use itertools::Itertools;
use ordermap::OrderMap;
use ratatui::{
    layout::*,
    prelude::*,
    text::{Line, Span, ToSpan},
    widgets::{Block, Borders, Padding, Paragraph, Widget, Wrap},
};

use crate::{
    types::{Message, Node},
    ui::helpers::{StringExt, hops_to_spans, humanize_datetime, routing_error_to_span, short_name_to_span},
};

const REACTIONS_LINE_MAX_WIDTH: usize = 20;
const MESSAGE_MAX_HEIGHT: u16 = 6;

pub struct MessageWidget<'a> {
    node: &'a Node,
    message: &'a Message,
    replying_node: Option<&'a Node>,
    is_my_node: bool,
    is_selected: bool,
    is_highlighted: bool,
    reactions_line: Line<'a>,
    message_paragraph: Paragraph<'a>,
}

impl<'a> MessageWidget<'a> {
    pub fn new(
        node: &'a Node,
        message: &'a Message,
        reactions: Vec<&'a Message>,
        replying_node: Option<&'a Node>,
        replying_message: Option<&'a Message>,
        is_my_node: bool,
        is_selected: bool,
        is_highlighted: bool,
    ) -> Self {
        let message_paragraph = get_text_paragraph(message, replying_message);

        Self {
            node,
            message,
            replying_node,
            is_my_node,
            is_selected,
            is_highlighted,
            reactions_line: get_reactions_line(reactions),
            message_paragraph,
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
            .border_set(if self.is_selected {
                symbols::border::THICK
            } else {
                symbols::border::PLAIN
            })
            .border_style(Style::new().fg(if self.is_highlighted {
                Color::Cyan
            } else if self.is_selected {
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
        if let Some(rep_node) = self.replying_node {
            Line::from(vec![
                short_name_to_span(self.node, self.is_my_node),
                " → ".to_span().dark_gray(),
                short_name_to_span(rep_node, self.is_my_node).on_magenta(),
            ])
            .render(v0_h[0], buf);
        } else {
            Line::from(vec![
                short_name_to_span(self.node, self.is_my_node),
                " ".to_span(),
                self.node.long_name().to_span().bold(),
            ])
            .render(v0_h[0], buf);
        }

        if !self.is_my_node {
            Line::from(hops_to_spans(self.message, false)).render(v0_h[1], buf);
        } else {
            routing_error_to_span(self.message.routing_error).render(v0_h[1], buf);
        }

        Line::from(Span::from(humanize_datetime(self.message.datetime.with_timezone(&chrono::Local))).dark_gray())
            .right_aligned()
            .render(v0_h[2], buf);

        // second line
        self.message_paragraph.render(v1_h[0], buf);
        self.reactions_line.render(v1_h[2], buf);

        // thirs line
        if is_overheight {
            Line::from(
                [
                    Some(Span::from("(trimmed)").red().add_modifier(if self.is_selected {
                        Modifier::empty()
                    } else {
                        Modifier::DIM
                    })),
                    self.is_selected.then_some(Span::from(" press ").dark_gray().dim()),
                    self.is_selected.then_some(Span::from("<enter>").dim()),
                    self.is_selected.then_some(Span::from(" to expand").dark_gray().dim()),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<Span<'_>>>(),
            )
            .render(v[2], buf);
        }
    }
}

fn get_text_paragraph<'a>(message: &'a Message, replied_message: Option<&'a Message>) -> Paragraph<'a> {
    let reply_line = replied_message
        .map(|msg| Line::from(vec!["“".to_span(), Span::from(msg.text_oneline()), "”".to_span()]).magenta());

    let text_lines: Vec<Line<'_>> = message.text.to_hyperlinked_lines();

    Paragraph::new(reply_line.into_iter().chain(text_lines).collect::<Vec<Line<'_>>>()).wrap(Wrap { trim: false })
}

fn get_reactions_line(reactions: Vec<&'_ Message>) -> Line<'_> {
    let summary = reactions
        .iter()
        .sorted_by_key(|r| r.datetime)
        .fold(OrderMap::new(), |mut acc, r| {
            *acc.entry(&r.text).or_insert(0) += 1;
            acc
        });

    Line::from(
        summary
            .into_iter()
            .flat_map(|(emoji, nodes_count)| {
                if nodes_count > 1 {
                    vec![
                        " ".to_span(),
                        emoji.to_span(),
                        Span::from(format!(":{}", nodes_count)).dark_gray(),
                    ]
                } else {
                    vec![" ".to_span(), emoji.to_span()]
                }
            })
            .collect::<Vec<Span>>(),
    )
    .right_aligned()
}
