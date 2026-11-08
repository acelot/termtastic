use std::time::Duration;

use ratatui::prelude::*;
use ratatui::widgets::{LineGauge, Paragraph};

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
        let h = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(2)]).split(area);

        Paragraph::new(Line::from(vec![
            Span::from(crate::APP_NAME).magenta().bold(),
            Span::from(" "),
            Span::from(crate::APP_VERSION).dark_gray(),
        ]))
        .render(h[0], buf);

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

        match self.context.connection_state {
            ConnectionState::NotConnected => {
                Line::from(vec![Span::from("not connected").dark_gray()])
                    .right_aligned()
                    .render(h[1], buf);
            }
            ConnectionState::ProblemDetected { .. } => {
                Line::from(vec![
                    Span::from(format!(
                        "reconnecting in {} sec...",
                        self.context
                            .reconnection_backoff
                            .map(|b| b.as_secs().to_string())
                            .unwrap_or("?".to_owned())
                    ))
                    .red(),
                ])
                .right_aligned()
                .render(h[1], buf);
            }
            ConnectionState::Connecting => {
                Line::from(vec![Span::from("connecting...").yellow()])
                    .right_aligned()
                    .render(h[1], buf);
            }
            ConnectionState::LoadingDbData => {
                Line::from(vec![Span::from("loading DB...").yellow()])
                    .right_aligned()
                    .render(h[1], buf);
            }
            ConnectionState::LoadingNodeConfig => {
                Line::from(vec![
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
                ])
                .right_aligned()
                .render(h[1], buf);
            }
            ConnectionState::Connected => {
                let status_line = Line::from(vec![
                    Span::from("online ").dark_gray(),
                    Span::from(self.context.online_nodes.to_string()).green(),
                    Span::from("/").dark_gray().dim(),
                    Span::from(self.context.nodes_count.to_string()).dark_gray(),
                    Span::from("  "),
                    db_indicator,
                    Span::from(" "),
                    tr_indicator,
                    Span::from(" "),
                    rx_indicator,
                ]);

                let h1_h = Layout::horizontal([
                    Constraint::Min(0),
                    Constraint::Length(4),
                    Constraint::Length(11),
                    Constraint::Length(2),
                    Constraint::Length(6),
                    Constraint::Length(2),
                    Constraint::Length(status_line.width() as u16),
                ])
                .split(h[1]);

                if let Some(battery_level) = self.context.battery_level {
                    if battery_level > 100 {
                        Line::from(vec![Span::from("\u{1F50C} PWR").dark_gray()])
                            .right_aligned()
                            .render(h1_h[2], buf);
                    } else {
                        let color = battery_level.battery_level_to_color();

                        Line::from(vec![Span::from(format!("{}%", battery_level)).fg(color)])
                            .right_aligned()
                            .render(h1_h[1], buf);

                        LineGauge::default()
                            .unfilled_symbol("\u{258C}")
                            .unfilled_style(Style::new().dark_gray())
                            .filled_symbol("\u{258C}")
                            .filled_style(Style::new().fg(color))
                            .ratio(battery_level as f64 / 100.0)
                            .label("")
                            .render(h1_h[2], buf);
                    }
                }

                if let Some(my_node) = self.context.my_node {
                    short_name_to_span(my_node, true).render(h1_h[4], buf);
                }

                status_line.render(h1_h[6], buf);
            }
        };
    }
}
