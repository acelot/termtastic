use meshtastic::protobufs::HardwareModel;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Padding},
};

use crate::{
    state::State,
    types::{Device, Node},
    ui::{
        helpers::{BatteryLevelColorExt, short_name_to_span},
        widget::{DeviceWidget, PlaceholderWidget},
    },
};

pub struct MyNodeWidget<'a> {
    data: MyNodeWidgetData<'a>,
}

impl<'a> MyNodeWidget<'a> {
    pub fn new(data: MyNodeWidgetData<'a>) -> Self {
        Self { data }
    }
}

impl<'a> Widget for MyNodeWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::new()
            .borders(Borders::all())
            .border_type(BorderType::Rounded)
            .border_style(Style::new().dark_gray())
            .padding(Padding::symmetric(1, 0))
            .title_top(" node ");

        let block_area = block.inner(area);
        block.render(area, buf);

        let Some(node) = self.data.node else {
            PlaceholderWidget::dark_gray("not connected").render(block_area, buf);
            return;
        };

        let v = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(if block_area.height > 8 { 1 } else { 0 }),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(block_area);

        let v6_h = Layout::horizontal([Constraint::Fill(1), Constraint::Length(1), Constraint::Fill(2)]).split(v[6]);

        if let Some(device) = self.data.device {
            DeviceWidget::new(device, false, false).render(v[0], buf);
        }

        Line::from(short_name_to_span(node, true)).centered().render(v[2], buf);

        if let Some(hw_model) = self.data.hw_model {
            Line::from(Span::from(hw_model).magenta()).centered().render(v[4], buf);
        }

        if let Some(battery_level) = self.data.battery_level {
            let span = if battery_level > 100 {
                Span::from("\u{1F50C} PWR").dark_gray()
            } else {
                Span::from(format!("BAT {}%", battery_level)).fg(battery_level.battery_level_to_color())
            };

            Line::from(span).render(v6_h[0], buf);
        }

        if let Some(firmware_version) = self.data.firmware_version {
            Line::from(vec![
                Span::from("v").dark_gray(),
                Span::from(firmware_version).dark_gray(),
            ])
            .right_aligned()
            .render(v6_h[2], buf);
        }
    }
}

pub struct MyNodeWidgetData<'a> {
    node: Option<&'a Node>,
    device: Option<&'a Device>,
    hw_model: Option<&'a str>,
    firmware_version: Option<&'a String>,
    battery_level: Option<u32>,
}

impl<'a> From<&'a State> for MyNodeWidgetData<'a> {
    fn from(value: &'a State) -> Self {
        let device_metadata = value.device_metadata.as_ref();

        Self {
            node: value.my_node_key.and_then(|key| value.nodes.get(&key)),
            device: value.active_device.as_ref(),
            hw_model: device_metadata.and_then(|m| {
                Some(
                    HardwareModel::try_from(m.hw_model)
                        .ok()
                        .map(|h| h.as_str_name())
                        .unwrap_or("UNKNOWN"),
                )
            }),
            firmware_version: device_metadata.and_then(|m| Some(&m.firmware_version)),
            battery_level: value
                .my_node_key
                .and_then(|my_node_key| value.nodes_last_telemetry.get(&my_node_key))
                .and_then(|telemetry| telemetry.device_metrics.as_ref())
                .and_then(|device_metrics| device_metrics.data.battery_level),
        }
    }
}
