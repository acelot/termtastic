use std::sync::LazyLock;

use chrono::Local;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders},
};

use crate::ui::widget::FontWidget;

#[rustfmt::skip]
static COLON_LARGE: LazyLock<String> = LazyLock::new(|| [
    "   ",
    " ▪ ",
    "   ",
    " ▪ ",
    "   ",
].join("\n"));

#[rustfmt::skip]
static COLON_SMALL: LazyLock<String> = LazyLock::new(|| [
    " ", 
    ":", 
    " "
].join("\n"));

pub struct ClockWidget;

impl Widget for ClockWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::new()
            .borders(Borders::all())
            .border_type(BorderType::Plain)
            .border_style(Style::new().dark_gray())
            .title_top(" clock ");

        let block_area = block.inner(area);
        block.render(area, buf);

        let now = Local::now();
        let hour = now.format("%H").to_string();
        let minute = now.format("%M").to_string();

        let hour_widget = if block_area.height > 8 {
            FontWidget::large_bold(&hour, 1)
        } else {
            FontWidget::medium_bold(&hour, 0)
        };

        let minute_widget = if block_area.height > 8 {
            FontWidget::large_bold(&minute, 1)
        } else {
            FontWidget::medium_bold(&minute, 0)
        };

        let (colon_widget, colon_widget_width) = if block_area.height > 8 {
            (Text::from(COLON_LARGE.as_str()), 3)
        } else {
            (Text::from(COLON_SMALL.as_str()), 1)
        };

        let (hour_widget_width, hour_widget_height) = hour_widget.size();
        let (minute_widget_width, _) = minute_widget.size();

        let v = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(hour_widget_height),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(block_area);

        let v1_h = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(hour_widget_width),
            Constraint::Length(colon_widget_width),
            Constraint::Length(minute_widget_width),
            Constraint::Min(0),
        ])
        .split(v[1]);

        hour_widget.render(v1_h[1], buf);
        colon_widget.render(v1_h[2], buf);
        minute_widget.render(v1_h[3], buf);

        Line::from(Span::from(now.format("%e %B %Y").to_string()).dark_gray())
            .centered()
            .render(v[3], buf);
    }
}
