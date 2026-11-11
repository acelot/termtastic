use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Padding},
};

use crate::{
    state::State,
    types::Node,
    ui::{
        helpers::short_name_to_span,
        widget::{FontWidget, PlaceholderWidget},
    },
};

pub enum NodeDistanceWidgetMode {
    Closest,
    Farthest,
}

pub struct NodeDistanceWidget<'a> {
    mode: NodeDistanceWidgetMode,
    data: &'a NodeDistanceWidgetData<'a>,
}

impl<'a> NodeDistanceWidget<'a> {
    pub fn new(mode: NodeDistanceWidgetMode, data: &'a NodeDistanceWidgetData<'a>) -> Self {
        Self { mode, data }
    }
}

impl<'a> Widget for NodeDistanceWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::new()
            .borders(Borders::all())
            .border_type(BorderType::Rounded)
            .border_style(Style::new().dark_gray())
            .padding(Padding::symmetric(1, 0))
            .title_top(match self.mode {
                NodeDistanceWidgetMode::Closest => " closest node ",
                NodeDistanceWidgetMode::Farthest => " farthest node ",
            });

        let block_area = block.inner(area);
        block.render(area, buf);

        let Some((node, distance)) = (match self.mode {
            NodeDistanceWidgetMode::Closest => self.data.closest,
            NodeDistanceWidgetMode::Farthest => self.data.farthest,
        }) else {
            PlaceholderWidget::dark_gray("no data").render(block_area, buf);
            return;
        };

        let (distance_formatted, unit_line) = if distance < 1000.0 {
            (format!("{:.0}", distance), Line::from("meters"))
        } else {
            (
                format!("{:.0}", distance / 1000.0),
                Line::from(vec![
                    Span::from("kilometers"),
                    Span::from(format!(" and {:.0} meters", distance % 1000.0)).dark_gray(),
                ]),
            )
        };

        let num_widget = if block_area.height > 8 {
            FontWidget::large_bold(&distance_formatted, 1)
        } else {
            FontWidget::medium_bold(&distance_formatted, 0)
        };

        let (num_widget_width, num_widget_height) = num_widget.size();

        let v = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(num_widget_height),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(block_area);

        let v2_h = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(num_widget_width),
            Constraint::Min(0),
        ])
        .split(v[2]);

        Line::from(vec![
            short_name_to_span(node, false),
            Span::from(" "),
            Span::from(node.long_name()),
        ])
        .render(v[0], buf);

        num_widget.render(v2_h[1], buf);

        unit_line.centered().render(v[4], buf);
    }
}

#[derive(Default)]
pub struct NodeDistanceWidgetData<'a> {
    closest: Option<(&'a Node, f32)>,
    farthest: Option<(&'a Node, f32)>,
}

impl<'a> From<&'a State> for NodeDistanceWidgetData<'a> {
    fn from(value: &'a State) -> Self {
        let Some(my_node_key) = value.my_node_key else {
            return Self::default();
        };

        let Some(my_coords) = value
            .nodes_last_position
            .get(&my_node_key)
            .and_then(|pos| Some(pos.coordinates()))
        else {
            return Self::default();
        };

        let mut closest_node: Option<&Node> = None;
        let mut closest_distance: f32 = f32::MAX;
        let mut farthest_node: Option<&Node> = None;
        let mut farthest_distance: f32 = 0.0;

        for (_, node) in value.nodes.iter() {
            if node.key == my_node_key {
                continue;
            }

            let Some(node_pos) = value.nodes_last_position.get(&node.key) else {
                continue;
            };

            let distance = distance_in_meters(my_coords, node_pos.coordinates());

            if distance > farthest_distance {
                farthest_node = Some(node);
                farthest_distance = distance;
            }

            if distance < closest_distance {
                closest_node = Some(node);
                closest_distance = distance;
            }
        }

        Self {
            closest: closest_node.and_then(|n| Some((n, closest_distance))),
            farthest: farthest_node.and_then(|n| Some((n, farthest_distance))),
        }
    }
}

fn distance_in_meters((lat1, lon1): (f32, f32), (lat2, lon2): (f32, f32)) -> f32 {
    // Mean Earth radius in meters.
    const EARTH_RADIUS_M: f64 = 6_371_000.0;

    let lat1 = f64::from(lat1).to_radians();
    let lon1 = f64::from(lon1).to_radians();
    let lat2 = f64::from(lat2).to_radians();
    let lon2 = f64::from(lon2).to_radians();

    // Haversine formula.
    let delta_lat = lat2 - lat1;
    let delta_lon = lon2 - lon1;

    let a = (delta_lat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);

    // `atan2` keeps the result well conditioned for antipodal points.
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    (EARTH_RADIUS_M * c) as f32
}
