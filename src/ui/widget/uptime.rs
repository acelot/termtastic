use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Padding},
};

use crate::{
    state::State,
    ui::widget::{FontWidget, PlaceholderWidget},
};

pub struct UptimeWidget {
    data: UptimeWidgetData,
}

impl UptimeWidget {
    pub fn new(data: UptimeWidgetData) -> Self {
        Self { data }
    }
}

impl Widget for UptimeWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::new()
            .borders(Borders::all())
            .border_type(BorderType::Plain)
            .border_style(Style::new().dark_gray())
            .padding(Padding::symmetric(1, 0))
            .title_top(" uptime ");

        let block_area = block.inner(area);
        block.render(area, buf);

        let Some(uptime) = self.data.uptime else {
            PlaceholderWidget::dark_gray("UNKNOWN").render(block_area, buf);
            return;
        };

        let (uptime, measure, addition) = match uptime {
            0..60 => (uptime, "seconds".to_owned(), None),
            60..120 => (1, "minute".to_owned(), None),
            120..3600 => (uptime / 60, "minutes".to_owned(), None),
            3600..7200 => (
                1,
                "hour".to_owned(),
                Some(format!("and {} minutes", uptime % 3600 / 60)),
            ),
            7200..86400 => (
                uptime / 3600,
                "hours".to_owned(),
                Some(format!("and {} minutes", uptime % 3600 / 60)),
            ),
            86400..172800 => (
                1,
                "day".to_owned(),
                Some(format!("and {} hours", uptime % 86400 / 3600)),
            ),
            _ => (
                uptime / 86400,
                "days".to_owned(),
                Some(format!("and {} hours", uptime % 86400 / 3600)),
            ),
        };

        let uptime_string = uptime.to_string();

        let num_widget = if block_area.height > 8 {
            FontWidget::large_bold(&uptime_string, 1)
        } else {
            FontWidget::medium_bold(&uptime_string, 0)
        };

        let (num_widget_width, num_widget_height) = num_widget.size();

        let v = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(num_widget_height),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(block_area);

        let v1_h = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(num_widget_width),
            Constraint::Min(0),
        ])
        .split(v[1]);

        num_widget.render(v1_h[1], buf);

        Line::from(
            [
                Some(Span::from(measure)),
                addition.and_then(|add| Some(Span::from(format!(" {}", add)).dark_gray())),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<Span<'_>>>(),
        )
        .centered()
        .render(v[3], buf);
    }
}

pub struct UptimeWidgetData {
    uptime: Option<u32>,
}

impl From<&State> for UptimeWidgetData {
    fn from(value: &State) -> Self {
        Self {
            uptime: value
                .my_node_key
                .and_then(|key| value.nodes_last_telemetry.get(&key))
                .and_then(|t| t.device_metrics.as_ref())
                .and_then(|m| m.data.uptime_seconds),
        }
    }
}
