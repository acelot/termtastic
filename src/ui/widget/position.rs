use chrono::Utc;
use meshtastic::protobufs::position::LocSource;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Widget},
};

use crate::{types::NodePosition, ui::helpers::humanize_time_delta};

pub struct PositionWidget<'a> {
    pub item: &'a NodePosition,
    pub is_selected: bool,
}

impl<'a> Widget for PositionWidget<'a> {
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

        let v0_h = Layout::horizontal([Constraint::Fill(2), Constraint::Length(1), Constraint::Fill(1)]).split(v[0]);
        let v1_h = Layout::horizontal(Constraint::from_fills([1, 1, 1])).split(v[1]);

        // first line
        Line::from(vec![
            match self.item.position.location_source() {
                LocSource::LocUnset => Span::from("POS ").magenta(),
                LocSource::LocManual => Span::from("FIX ").dark_gray(),
                LocSource::LocInternal => Span::from("INT ").green(),
                LocSource::LocExternal => Span::from("EXT ").blue(),
            },
            Span::from(self.item.coordinates_as_str()),
        ])
        .render(v0_h[0], buf);

        Line::from(humanize_time_delta(
            Utc::now().signed_duration_since(self.item.datetime),
        ))
        .right_aligned()
        .render(v0_h[2], buf);

        // second line
        Line::from(vec![
            Span::from("alt ").dark_gray(),
            if let Some(alt) = self.item.position.altitude {
                Span::from(format!("{} m", alt))
            } else {
                Span::from("no data").dark_gray().dim()
            },
        ])
        .render(v1_h[0], buf);

        Line::from(vec![
            Span::from("acc ").dark_gray(),
            if self.item.position.gps_accuracy > 0 {
                Span::from(format!("{} m", self.item.position.gps_accuracy))
            } else {
                Span::from("no data").dark_gray().dim()
            },
        ])
        .centered()
        .render(v1_h[1], buf);

        Line::from(vec![
            Span::from("sat ").dark_gray(),
            if self.item.position.sats_in_view > 0 {
                Span::from(self.item.position.sats_in_view.to_string())
            } else {
                Span::from("no data").dark_gray().dim()
            },
        ])
        .right_aligned()
        .render(v1_h[2], buf);
    }
}
