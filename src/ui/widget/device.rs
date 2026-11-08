use itertools::Itertools;
use ratatui::prelude::*;

use crate::types::Device;

pub struct DeviceWidget<'a> {
    device: &'a Device,
    is_selected: bool,
    centered: bool,
}

impl<'a> DeviceWidget<'a> {
    pub fn new(device: &'a Device, is_selected: bool, centered: bool) -> Self {
        Self {
            device,
            is_selected,
            centered,
        }
    }
}

impl<'a> Widget for DeviceWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let modifier = if self.is_selected {
            Modifier::REVERSED
        } else {
            Modifier::empty()
        };

        let spans = match self.device {
            Device::Ble { name, address, id } => vec![
                Span::from(" BLE ").black().on_blue(),
                Span::from(format!(
                    " {} ",
                    [name, id, &Some(address.to_string())].into_iter().flatten().join(" – ")
                ))
                .add_modifier(modifier),
            ],
            Device::Tcp { name, address } => vec![
                Span::from(" TCP ").black().on_green(),
                Span::from(format!(
                    " {} ",
                    [name, &Some(address.to_string())].into_iter().flatten().join(" – ")
                ))
                .add_modifier(modifier),
            ],
            Device::Serial { name, address } => vec![
                Span::from(" COM ").black().on_magenta(),
                Span::from(format!(
                    " {} ",
                    [name, &Some(address.to_string())].into_iter().flatten().join(" – ")
                ))
                .add_modifier(modifier),
            ],
        };

        let mut line = Line::from(spans);

        if self.centered {
            line = line.centered();
        }

        line.render(area, buf);
    }
}
