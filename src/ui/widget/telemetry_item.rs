use chrono::Utc;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Stylize},
    text::{Line, Span},
    widgets::Widget,
};

use crate::{types::TelemetryItem, ui::helpers::humanize_time_delta};

pub struct TelemetryItemWidget<'a> {
    pub item: &'a TelemetryItem,
    pub is_selected: bool,
}

impl<'a> Widget for TelemetryItemWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.item {
            TelemetryItem::Group { title, datetime, .. } => {
                let h =
                    Layout::horizontal([Constraint::Fill(3), Constraint::Fill(2), Constraint::Length(2)]).split(area);

                Line::from(vec![Span::from(title).bold().add_modifier(if self.is_selected {
                    Modifier::UNDERLINED
                } else {
                    Modifier::empty()
                })])
                .magenta()
                .render(h[0], buf);

                Line::from(humanize_time_delta(Utc::now().signed_duration_since(datetime)))
                    .right_aligned()
                    .render(h[1], buf);
            }
            TelemetryItem::Item {
                title, formatted_value, ..
            } => {
                let v = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(area);

                Line::from(vec![
                    Span::from("  "),
                    Span::from(format!("{}:", title)).add_modifier(if self.is_selected {
                        Modifier::UNDERLINED | Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
                ])
                .render(v[0], buf);

                formatted_value
                    .as_ref()
                    .map(Span::from)
                    .unwrap_or(Span::from("no data").dark_gray())
                    .add_modifier(if self.is_selected {
                        Modifier::UNDERLINED | Modifier::BOLD
                    } else {
                        Modifier::empty()
                    })
                    .render(v[1], buf);
            }
        }
    }
}
