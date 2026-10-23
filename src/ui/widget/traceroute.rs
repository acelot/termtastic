use chrono::Utc;
use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Widget},
};

use crate::{
    types::{Traceroute, TracerouteState},
    ui::helpers::{hops_to_spans, humanize_time_delta, humanize_uptime, routing_error_to_span},
};

pub struct TracerouteWidget<'a> {
    pub item: &'a Traceroute,
    pub is_selected: bool,
}

impl<'a> Widget for TracerouteWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = Rect::new(area.x, area.y, area.width, 2);

        let block = Block::new()
            .padding(Padding::new(1, 3, 0, 0))
            .borders(Borders::LEFT)
            .border_type(if self.is_selected {
                BorderType::Thick
            } else {
                BorderType::Plain
            })
            .border_style(Style::new().fg(if self.is_selected {
                Color::Yellow
            } else {
                Color::DarkGray
            }));

        let block_area = block.inner(area);
        block.render(area, buf);

        let v = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(block_area);
        let v0_h = Layout::horizontal([Constraint::Fill(4), Constraint::Length(1), Constraint::Fill(1)]).split(v[0]);
        let v1_h = Layout::horizontal([
            Constraint::Fill(2),
            Constraint::Length(1),
            Constraint::Fill(2),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .split(v[1]);

        // state
        (match &self.item.state {
            TracerouteState::Started => Span::from("pending...").yellow(),
            TracerouteState::RoutingError => routing_error_to_span(self.item.routing_error),
            TracerouteState::TimedOut => Span::from("timed out").dark_gray(),
            TracerouteState::Finished => Span::from("finished").green(),
        })
        .add_modifier(if self.is_selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        })
        .render(v0_h[0], buf);

        // date
        Line::from(humanize_time_delta(
            Utc::now().signed_duration_since(self.item.datetime),
        ))
        .right_aligned()
        .render(v0_h[2], buf);

        // towards
        Line::from(
            vec![Span::from("twrd ").dark_gray()]
                .into_iter()
                .chain(if !self.item.route_towards.is_empty() {
                    hops_to_spans(&self.item.route_towards, false)
                } else {
                    vec![Span::from("no data").dark_gray().dim()]
                })
                .collect_vec(),
        )
        .render(v1_h[0], buf);

        // back
        Line::from(
            vec![Span::from("back ").dark_gray()]
                .into_iter()
                .chain(if !self.item.route_back.is_empty() {
                    hops_to_spans(&self.item.route_back, false)
                } else {
                    vec![Span::from("no data").dark_gray().dim()]
                })
                .collect_vec(),
        )
        .render(v1_h[2], buf);

        // time
        Line::from(vec![
            Span::from("rtt ").dark_gray(),
            Span::from(humanize_uptime(if self.item.state == TracerouteState::Started {
                Utc::now()
                    .signed_duration_since(self.item.datetime)
                    .num_seconds()
                    .max(0) as u32
            } else {
                self.item.duration.num_seconds() as u32
            })),
        ])
        .right_aligned()
        .render(v1_h[4], buf);
    }
}
