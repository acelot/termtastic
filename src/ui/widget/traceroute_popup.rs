use std::{collections::HashMap, iter};

use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, ScrollbarState, StatefulWidget, Widget, Wrap},
};

use crate::{
    types::{Node, Traceroute, TracerouteItem, TracerouteState, UNKNOWN_NODE},
    ui::{
        helpers::{SnrColorExt, default_scrollbar, humanize_uptime, routing_error_to_span, short_name_to_span},
        widget::PlaceholderWidget,
    },
};

pub struct TraceroutePopupWidget<'a> {
    pub traceroute: &'a Traceroute,
    pub nodes: &'a HashMap<u32, Node>,
    pub my_node_key: u32,
}

#[derive(Debug, Clone, Default)]
pub struct TraceroutePopupWidgetState {
    expanded_traceroute_scroll_state: ScrollbarState,
}

impl TraceroutePopupWidgetState {
    pub fn scroll_to_top(&mut self) {
        self.expanded_traceroute_scroll_state.first();
    }

    pub fn scroll_up(&mut self) {
        self.expanded_traceroute_scroll_state.prev();
    }

    pub fn scroll_down(&mut self) {
        self.expanded_traceroute_scroll_state.next();
    }
}

impl<'a> StatefulWidget for TraceroutePopupWidget<'a> {
    type State = TraceroutePopupWidgetState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let popup_area = Rect {
            x: area.x - 1,
            y: area.y + 2,
            width: area.width + 2,
            height: area.height - 1,
        };

        let popup_block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .padding(Padding::symmetric(1, 0));

        let popup_block_area = popup_block.inner(popup_area);

        Clear.render(popup_area.outer(Margin::new(1, 0)), buf);
        popup_block.render(popup_area, buf);

        let route_span_mapper = |item: &TracerouteItem| match item {
            TracerouteItem::Node(node_key) => {
                let node = self.nodes.get(&node_key).unwrap_or(&UNKNOWN_NODE);
                short_name_to_span(node, node_key == &self.my_node_key)
            }
            TracerouteItem::Snr(snr) => Span::from(format!("{:.2}dB", snr)).style(Style::new().fg(snr.snr_to_color())),
        };

        #[allow(unstable_name_collisions)]
        let towards_spans: Vec<Span<'_>> = iter::once(&TracerouteItem::Node(self.my_node_key))
            .chain(self.traceroute.route_towards.iter())
            .chain(iter::once(&TracerouteItem::Node(self.traceroute.node_key)))
            .map(route_span_mapper)
            .intersperse(Span::from(" \u{2192}\u{A0}").dark_gray())
            .collect();

        #[allow(unstable_name_collisions)]
        let back_spans: Vec<Span<'_>> = iter::once(&TracerouteItem::Node(self.traceroute.node_key))
            .chain(self.traceroute.route_back.iter())
            .chain(iter::once(&TracerouteItem::Node(self.my_node_key)))
            .map(route_span_mapper)
            .intersperse(Span::from(" \u{2192}\u{A0}").dark_gray())
            .collect();

        match self.traceroute.state {
            TracerouteState::Started => {
                PlaceholderWidget::yellow("pending...").render(popup_block_area, buf);
            }
            TracerouteState::RoutingError => {
                PlaceholderWidget::red(routing_error_to_span(self.traceroute.routing_error))
                    .render(popup_block_area, buf);
            }
            TracerouteState::TimedOut => {
                PlaceholderWidget::dark_gray("timed out").render(popup_block_area, buf);
            }
            TracerouteState::Finished => {
                let h = Layout::horizontal([Constraint::Fill(1), Constraint::Length(1), Constraint::Length(1)])
                    .split(popup_block_area);

                let paragraph = Paragraph::new(vec![
                    Line::from(Span::from("ROUTE TOWARDS").magenta()),
                    Line::from(towards_spans),
                    Line::from(Span::from("")),
                    Line::from(Span::from("ROUTE BACK").magenta()),
                    Line::from(back_spans),
                    Line::from(Span::from("")),
                    Line::from(Span::from("ROUND TRIP TIME").magenta()),
                    Line::from(Span::from(humanize_uptime(
                        self.traceroute.duration.num_seconds() as u32
                    ))),
                ])
                .wrap(Wrap { trim: false });

                let paragraph_lines = paragraph.line_count(popup_block_area.width - 2);

                state.expanded_traceroute_scroll_state = state
                    .expanded_traceroute_scroll_state
                    .content_length(paragraph_lines.saturating_sub(popup_block_area.height as usize) + 1);

                paragraph
                    .scroll((state.expanded_traceroute_scroll_state.get_position() as u16, 0))
                    .render(h[0], buf);

                default_scrollbar().render(h[2], buf, &mut state.expanded_traceroute_scroll_state);
            }
        };
    }
}
