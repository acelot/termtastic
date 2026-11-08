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

use crate::{state::State, types::NodeTelemetryTyped, ui::widget::PlaceholderWidget};

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
            .border_type(BorderType::Plain)
            .border_style(Style::new().dark_gray())
            .padding(Padding::symmetric(1, 0))
            .title_top(Line::from(vec![
                Span::from(" device metrics ("),
                Span::from("channel util").green(),
                Span::from(", "),
                Span::from("air util tx").yellow(),
                Span::from(") "),
            ]));

        let block_area = block.inner(area);
        block.render(area, buf);

        if self.data.is_empty() {
            PlaceholderWidget::dark_gray("no data").render(block_area, buf);
            return;
        }

        let (datetime_min, datetime_max) = (Utc::now().sub(Duration::from_hours(24)), Utc::now());

        let (ch_util, air_util_tx, max): (Vec<(f64, f64)>, Vec<(f64, f64)>, f64) = self.data.iter().fold(
            (Vec::new(), Vec::new(), 0.0),
            |(mut ch_util, mut air_util_tx, mut max), item| {
                let ch_util_f64 = item.data.channel_utilization() as f64;
                let air_util_tx_f64 = item.data.air_util_tx() as f64;

                ch_util.push((item.datetime.timestamp() as f64, ch_util_f64));
                air_util_tx.push((item.datetime.timestamp() as f64, air_util_tx_f64));

                if ch_util_f64 > max {
                    max = ch_util_f64;
                }

                if air_util_tx_f64 > max {
                    max = air_util_tx_f64;
                }

                (ch_util, air_util_tx, max)
            },
        );

        let dataset_ch_util = Dataset::default()
            .name("channel util")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Color::Green)
            .data(ch_util.as_slice());

        let dataset_air_util_tx = Dataset::default()
            .name("air util tx")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Color::Yellow)
            .data(air_util_tx.as_slice());

        let x_axis = Axis::default()
            .bounds([datetime_min.timestamp() as f64, datetime_max.timestamp() as f64])
            .labels([
                datetime_min.with_timezone(&Local).format("%H:%M").to_string(),
                datetime_min
                    .add(Duration::from_hours(6))
                    .with_timezone(&Local)
                    .format("%H:%M")
                    .to_string(),
                datetime_min
                    .add(Duration::from_hours(12))
                    .with_timezone(&Local)
                    .format("%H:%M")
                    .to_string(),
                datetime_min
                    .add(Duration::from_hours(18))
                    .with_timezone(&Local)
                    .format("%H:%M")
                    .to_string(),
                datetime_max.with_timezone(&Local).format("%H:%M").to_string(),
            ]);

        let mut y_axis = Axis::default().title("%".blue());

        if max <= 25.0 {
            y_axis = y_axis.bounds([0.0, 25.0]).labels(["0", "5", "10", "15", "20", "25"]);
        } else if max <= 50.0 {
            y_axis = y_axis.bounds([0.0, 50.0]).labels(["0", "25", "50"]);
        } else if max <= 75.0 {
            y_axis = y_axis.bounds([0.0, 75.0]).labels(["0", "25", "50", "75"]);
        } else {
            y_axis = y_axis.bounds([0.0, 100.0]).labels(["0", "25", "50", "75", "100"]);
        }

        Chart::new(vec![dataset_air_util_tx, dataset_ch_util])
            .x_axis(x_axis)
            .y_axis(y_axis)
            .render(block_area, buf);
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
