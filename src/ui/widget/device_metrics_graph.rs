use std::{
    collections::VecDeque,
    ops::{Add, Deref, Sub},
    time::Duration,
};

use chrono::{Local, Utc};
use meshtastic::protobufs::DeviceMetrics;
use ratatui::{
    prelude::*,
    symbols::Marker,
    widgets::{Axis, Block, BorderType, Borders, Chart, Dataset, GraphType, Padding},
};

use crate::{
    service::DEVICE_METRICS_TIME_LIMIT_HOURS, state::State, types::NodeTelemetryTyped, ui::widget::PlaceholderWidget,
};

pub struct DeviceMetricsGraphWidget<'a> {
    data: DeviceMetricsGraphWidgetData<'a>,
}

impl<'a> DeviceMetricsGraphWidget<'a> {
    pub fn new(data: DeviceMetricsGraphWidgetData<'a>) -> Self {
        Self { data }
    }
}

impl<'a> Widget for DeviceMetricsGraphWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::new()
            .borders(Borders::all())
            .border_type(BorderType::Rounded)
            .border_style(Style::new().dark_gray())
            .padding(Padding::symmetric(1, 0))
            .title_top(" device metrics ");

        let block_area = block.inner(area);
        block.render(area, buf);

        if self.data.is_empty() {
            PlaceholderWidget::dark_gray("no data").render(block_area, buf);
            return;
        }

        let v = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).split(block_area);

        let (datetime_min, datetime_max) = (
            Utc::now().sub(Duration::from_hours(DEVICE_METRICS_TIME_LIMIT_HOURS)),
            Utc::now(),
        );

        let (ch_util, air_util_tx, util_max): (Vec<(f64, f64)>, Vec<(f64, f64)>, f64) = self.data.iter().fold(
            (Vec::new(), Vec::new(), 0.0),
            |(mut ch_util, mut air_util_tx, mut util_max), item| {
                let ch_util_f64 = item.data.channel_utilization() as f64;
                let air_util_tx_f64 = item.data.air_util_tx() as f64;

                ch_util.push((item.datetime.timestamp() as f64, ch_util_f64));
                air_util_tx.push((item.datetime.timestamp() as f64, air_util_tx_f64));

                util_max = util_max.max(ch_util_f64).max(air_util_tx_f64);

                (ch_util, air_util_tx, util_max)
            },
        );

        let current = self.data.iter().last().unwrap();

        let dataset_ch_util = Dataset::default()
            .name("channel util")
            .marker(Marker::Braille)
            .graph_type(GraphType::Scatter)
            .style(Color::Green)
            .data(ch_util.as_slice());

        let dataset_air_util_tx = Dataset::default()
            .name("air util tx")
            .marker(Marker::Braille)
            .graph_type(GraphType::Scatter)
            .style(Color::Yellow)
            .data(air_util_tx.as_slice());

        let x_axis_parts = if block_area.width < 100 {
            4
        } else if block_area.width < 120 {
            6
        } else {
            8
        };

        let x_axis = Axis::default()
            .bounds([datetime_min.timestamp() as f64, datetime_max.timestamp() as f64])
            .labels((0..=x_axis_parts).map(|i| {
                datetime_min
                    .add(Duration::from_hours(DEVICE_METRICS_TIME_LIMIT_HOURS * i / x_axis_parts))
                    .with_timezone(&Local)
                    .format("%H:%M")
                    .to_string()
            }));

        let y_axis_max = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0]
            .into_iter()
            .find(|&max| util_max <= max)
            .unwrap_or(100.0);

        let y_axis = Axis::default()
            .title("%".blue())
            .bounds([0.0, y_axis_max])
            .labels((0..=2).map(|i| format!("{}", y_axis_max / 2.0 * i as f64)));

        Chart::new(vec![dataset_ch_util, dataset_air_util_tx])
            .x_axis(x_axis)
            .y_axis(y_axis)
            .render(v[0], buf);

        Line::from(vec![
            Span::from("· channel util").green(),
            Span::from(" (current=").dark_gray(),
            Span::from(format!("{:.2}%", current.data.channel_utilization())),
            Span::from(")   ").dark_gray(),
            Span::from("· air util tx").yellow(),
            Span::from(" (current=").dark_gray(),
            Span::from(format!("{:.2}%", current.data.air_util_tx())),
            Span::from(")").dark_gray(),
        ])
        .render(v[1], buf);
    }
}

pub struct DeviceMetricsGraphWidgetData<'a>(&'a VecDeque<NodeTelemetryTyped<DeviceMetrics>>);

impl<'a> Deref for DeviceMetricsGraphWidgetData<'a> {
    type Target = VecDeque<NodeTelemetryTyped<DeviceMetrics>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<&'a State> for DeviceMetricsGraphWidgetData<'a> {
    fn from(value: &'a State) -> Self {
        Self(&value.my_node_device_metrics)
    }
}
