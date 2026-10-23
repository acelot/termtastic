use chrono::Local;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Clear, Padding, Row, ScrollbarState, StatefulWidget, Table, Widget},
};

use crate::{types::NodePosition, ui::helpers::default_scrollbar};

pub struct PositionPopupWidget<'a> {
    pub position: &'a NodePosition,
}

#[derive(Debug, Clone, Default)]
pub struct PositionPopupWidgetState {
    scroll_state: ScrollbarState,
}

impl PositionPopupWidgetState {
    pub fn scroll_to_top(&mut self) {
        self.scroll_state.first();
    }

    pub fn scroll_up(&mut self) {
        self.scroll_state.prev();
    }

    pub fn scroll_down(&mut self) {
        self.scroll_state.next();
    }
}

impl<'a> PositionPopupWidget<'a> {
    fn rows(&self) -> Vec<(&'static str, Span<'a>)> {
        let p = &self.position.position;

        let (lat, lon) = self.position.coordinates();

        let optional = |v: Option<i32>| match v {
            Some(v) => Span::from(v.to_string()),
            None => Span::from("none").dark_gray().dim(),
        };

        let optional_u32 = |v: Option<u32>| match v {
            Some(v) => Span::from(v.to_string()),
            None => Span::from("none").dark_gray(),
        };

        vec![
            (
                "datetime",
                Span::from(
                    self.position
                        .datetime
                        .with_timezone(&Local)
                        .format("%d.%m.%Y %H:%M:%S")
                        .to_string(),
                ),
            ),
            ("latitude", Span::from(format!("{:.5}", lat))),
            ("longitude", Span::from(format!("{:.5}", lon))),
            ("latitude", optional(p.altitude)),
            ("location source", Span::from(p.location_source().as_str_name())),
            ("altitude source", Span::from(p.altitude_source().as_str_name())),
            ("timestamp", Span::from(p.timestamp.to_string())),
            ("timestamp adjust", Span::from(p.timestamp_millis_adjust.to_string())),
            ("altitude HAE", optional(p.altitude_hae)),
            ("geoidal separation", optional(p.altitude_geoidal_separation)),
            ("PDOP", Span::from(p.pdop.to_string())),
            ("HDOP", Span::from(p.hdop.to_string())),
            ("VDOP", Span::from(p.vdop.to_string())),
            ("accuracy", Span::from(p.gps_accuracy.to_string())),
            ("ground speed", optional_u32(p.ground_speed)),
            ("ground track", optional_u32(p.ground_track)),
            ("fix quality", Span::from(p.fix_quality.to_string())),
            ("fix type", Span::from(p.fix_type.to_string())),
            ("sats in view", Span::from(p.sats_in_view.to_string())),
            ("sensor ID", Span::from(p.sensor_id.to_string())),
            ("next update", Span::from(p.next_update.to_string())),
            ("seq number", Span::from(p.seq_number.to_string())),
            ("precision bits", Span::from(p.precision_bits.to_string())),
        ]
    }
}

impl<'a> StatefulWidget for PositionPopupWidget<'a> {
    type State = PositionPopupWidgetState;

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

        let h = Layout::horizontal([Constraint::Fill(1), Constraint::Length(1), Constraint::Length(1)])
            .split(popup_block_area);

        let rows = self.rows();
        let key_width = rows.iter().map(|(k, _)| k.len()).max().unwrap_or(0) as u16;

        state.scroll_state = state
            .scroll_state
            .content_length(rows.len().saturating_sub(popup_block_area.height as usize) + 1);

        let table_rows = rows
            .into_iter()
            .skip(state.scroll_state.get_position())
            .map(|(key, value)| Row::new([Cell::from(Line::from(key).magenta()), Cell::from(Line::from(value))]));

        let table = Table::new(table_rows, [Constraint::Length(key_width), Constraint::Fill(1)]).column_spacing(3);

        Widget::render(table, h[0], buf);

        default_scrollbar().render(h[2], buf, &mut state.scroll_state);
    }
}
