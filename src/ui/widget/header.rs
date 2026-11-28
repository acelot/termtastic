use std::time::Duration;

use ratatui::prelude::*;

use crate::{
    state::State,
    types::{ConnectionState, DbLoadState, Node, TracerouteIndicatorState},
    ui::helpers::{BatteryLevelColorExt, short_name_to_span},
};

#[derive(Debug, Clone)]
pub struct HeaderContext<'a> {
    pub db_load_state: &'a DbLoadState,
    pub db_indicator: bool,
    pub traceroute_indicator: TracerouteIndicatorState,
    pub rx_indicator: bool,
    pub connection_state: &'a ConnectionState,
    pub reconnection_backoff: Option<Duration>,
    pub nodes_loader_current: u32,
    pub nodes_loader_total: u32,
    pub online_nodes: u32,
    pub nodes_count: usize,
    pub my_node: Option<&'a Node>,
    pub battery_level: Option<u32>,
    pub update_available: Option<&'a str>,
}

impl<'a> From<&'a State> for HeaderContext<'a> {
    fn from(state: &'a State) -> Self {
        let battery_level = state
            .my_node_key
            .and_then(|my_node_key| state.nodes_last_telemetry.get(&my_node_key))
            .and_then(|telemetry| telemetry.device_metrics.as_ref())
            .and_then(|device_metrics| device_metrics.data.battery_level);

        Self {
            db_load_state: &state.db_load_state,
            db_indicator: state.db_indicator,
            traceroute_indicator: state.traceroute_indicator,
            rx_indicator: state.rx_indicator,
            connection_state: &state.connection_state,
            reconnection_backoff: state.reconnection_backoff,
            nodes_loader_current: state.nodes_loader_current,
            nodes_loader_total: state.nodes_loader_total,
            online_nodes: state.online_nodes,
            nodes_count: state.nodes.len(),
            my_node: state.get_my_node(),
            battery_level,
            update_available: if !state.app_general_config.disable_update_check {
                state.update_available_version.as_deref()
            } else {
                None
            },
        }
    }
}

pub struct HeaderWidget<'a> {
    context: HeaderContext<'a>,
}

impl<'a> HeaderWidget<'a> {
    pub fn new(context: HeaderContext<'a>) -> Self {
        Self { context }
    }
}

impl Widget for HeaderWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let left_spans: Vec<Span<'_>> = [
            Some(Span::from(crate::APP_NAME).magenta().bold()),
            Some(Span::from(" ")),
            Some(Span::from(crate::APP_VERSION).dark_gray()),
            self.context
                .update_available
                .is_some()
                .then_some(Span::from("  new version ").dark_gray().dim()),
            self.context.update_available.map(|v| Span::from(v).dark_gray()),
        ]
        .into_iter()
        .flatten()
        .collect();

        let db_indicator = match self.context.db_load_state {
            DbLoadState::Failed => Span::from("DB_FAIL").red(),
            DbLoadState::Loaded => Span::from("DB")
                .fg(if self.context.db_indicator {
                    Color::Red
                } else {
                    Color::DarkGray
                })
                .add_modifier(if self.context.db_indicator {
                    Modifier::empty()
                } else {
                    Modifier::DIM
                }),
            _ => Span::from(""),
        };

        let tr_indicator = Span::from("TR")
            .fg(match self.context.traceroute_indicator {
                TracerouteIndicatorState::Off => Color::DarkGray,
                TracerouteIndicatorState::Running => Color::Yellow,
                TracerouteIndicatorState::Success => Color::Green,
                TracerouteIndicatorState::Error => Color::Red,
            })
            .add_modifier(if self.context.traceroute_indicator == TracerouteIndicatorState::Off {
                Modifier::DIM
            } else {
                Modifier::empty()
            });

        let rx_indicator = Span::from("RX")
            .fg(if self.context.rx_indicator {
                Color::Green
            } else {
                Color::DarkGray
            })
            .add_modifier(if self.context.rx_indicator {
                Modifier::empty()
            } else {
                Modifier::DIM
            });

        let right_spans = match self.context.connection_state {
            ConnectionState::NotConnected => {
                vec![Span::from("not connected").dark_gray()]
            }
            ConnectionState::ProblemDetected { .. } => {
                vec![
                    Span::from(format!(
                        "reconnecting in {} sec...",
                        self.context
                            .reconnection_backoff
                            .map(|b| b.as_secs().to_string())
                            .unwrap_or("?".to_owned())
                    ))
                    .red(),
                ]
            }
            ConnectionState::Connecting => {
                vec![Span::from("connecting...").yellow()]
            }
            ConnectionState::LoadingDbData => {
                vec![Span::from("loading DB...").yellow()]
            }
            ConnectionState::LoadingNodeConfig => {
                vec![
                    Span::from("loading nodes ").yellow(),
                    Span::from(self.context.nodes_loader_current.to_string()).yellow(),
                    Span::from("/").yellow().dim(),
                    Span::from(self.context.nodes_loader_total.to_string()).yellow(),
                    Span::from("  "),
                    db_indicator,
                    Span::from(" "),
                    Span::from("TR").dark_gray().dim(),
                    Span::from(" "),
                    rx_indicator,
                ]
            }
            ConnectionState::Connected => [
                if let Some(battery_level) = self.context.battery_level {
                    if battery_level > 100 {
                        Some(Span::from("ϟ PWR").dark_gray())
                    } else {
                        let color = battery_level.battery_level_to_color();

                        Some(Span::from(format!("BAT {}%", battery_level)).fg(color))
                    }
                } else {
                    None
                },
                self.context.battery_level.is_some().then_some(Span::from("  ")),
                self.context.my_node.map(|node| short_name_to_span(node, true)),
                self.context.my_node.is_some().then_some(Span::from("  ")),
                Some(Span::from("online ").dark_gray()),
                Some(Span::from(self.context.online_nodes.to_string()).green()),
                Some(Span::from("/").dark_gray().dim()),
                Some(Span::from(self.context.nodes_count.to_string()).dark_gray()),
                Some(Span::from("  ")),
                Some(db_indicator),
                Some(Span::from(" ")),
                Some(tr_indicator),
                Some(Span::from(" ")),
                Some(rx_indicator),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<Span<'_>>>(),
        };

        let right_line = Line::from(right_spans);

        let [left_area, _, right_area] = Layout::horizontal([
            Constraint::Min(10),
            Constraint::Length(1),
            Constraint::Length(right_line.width() as u16),
        ])
        .areas(area);

        Line::from(left_spans).render(left_area, buf);
        right_line.right_aligned().render(right_area, buf);
    }
}
