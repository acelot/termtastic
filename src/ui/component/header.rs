use crate::ui::prelude::*;

pub struct Header {}

impl Header {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for Header {
    fn get_hotkeys(&self, _state: &State) -> Vec<Hotkey> {
        Vec::default()
    }

    fn handle_event(&mut self, _state: &State, _event: &Event, _emit: &impl Fn(AppEvent)) -> bool {
        false
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        let h = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(2)]).split(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::from(crate::APP_NAME).magenta().bold(),
                Span::from(" "),
                Span::from(crate::APP_VERSION).dark_gray(),
            ])),
            h[0],
        );

        let db_indicator = match state.db_load_state {
            DbLoadState::Failed => Span::from("DB_FAIL").red(),
            DbLoadState::Loaded => Span::from("DB")
                .fg(if state.db_indicator {
                    Color::Red
                } else {
                    Color::DarkGray
                })
                .add_modifier(if state.db_indicator {
                    Modifier::empty()
                } else {
                    Modifier::DIM
                }),
            _ => Span::from(""),
        };

        let tr_indicator = Span::from("TR")
            .fg(match state.traceroute_indicator {
                TracerouteIndicatorState::Off => Color::DarkGray,
                TracerouteIndicatorState::Running => Color::Yellow,
                TracerouteIndicatorState::Success => Color::Green,
                TracerouteIndicatorState::Error => Color::Red,
            })
            .add_modifier(if state.traceroute_indicator == TracerouteIndicatorState::Off {
                Modifier::DIM
            } else {
                Modifier::empty()
            });

        let rx_indicator = Span::from("RX")
            .fg(if state.rx_indicator {
                Color::Green
            } else {
                Color::DarkGray
            })
            .add_modifier(if state.rx_indicator {
                Modifier::empty()
            } else {
                Modifier::DIM
            });

        match &state.connection_state {
            ConnectionState::NotConnected => {
                Line::from(vec![Span::from("not connected").dark_gray()])
                    .right_aligned()
                    .render(h[1], frame.buffer_mut());
            }
            ConnectionState::ProblemDetected { .. } => {
                Line::from(vec![
                    Span::from(format!(
                        "reconnecting in {} sec...",
                        state
                            .reconnection_backoff
                            .map(|b| b.as_secs().to_string())
                            .unwrap_or("?".to_owned())
                    ))
                    .red(),
                ])
                .right_aligned()
                .render(h[1], frame.buffer_mut());
            }
            ConnectionState::Connecting => {
                Line::from(vec![Span::from("connecting...").yellow()])
                    .right_aligned()
                    .render(h[1], frame.buffer_mut());
            }
            ConnectionState::LoadingDbData => {
                Line::from(vec![Span::from("loading DB...").yellow()])
                    .right_aligned()
                    .render(h[1], frame.buffer_mut());
            }
            ConnectionState::LoadingNodeConfig => {
                Line::from(vec![
                    Span::from("loading nodes ").yellow(),
                    Span::from(state.nodes_loader_current.to_string()).yellow(),
                    Span::from("/").yellow().dim(),
                    Span::from(state.nodes_loader_total.to_string()).yellow(),
                    Span::from("  "),
                    db_indicator,
                    Span::from(" "),
                    Span::from("TR").dark_gray().dim(),
                    Span::from(" "),
                    rx_indicator,
                ])
                .right_aligned()
                .render(h[1], frame.buffer_mut());
            }
            ConnectionState::Connected => {
                let status_line = Line::from(vec![
                    Span::from("online ").dark_gray(),
                    Span::from(state.online_nodes.to_string()).green(),
                    Span::from("/").green().dim(),
                    Span::from(state.nodes.len().to_string()).green(),
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

                if let Some(battery_level) = state
                    .my_node_key
                    .and_then(|my_node_key| state.nodes_last_telemetry.get(&my_node_key))
                    .and_then(|telemetry| telemetry.device_metrics.as_ref())
                    .and_then(|device_metrics| device_metrics.data.battery_level)
                {
                    if battery_level > 100 {
                        Line::from(vec![Span::from("\u{1F50C} PWR").dark_gray()])
                            .right_aligned()
                            .render(h1_h[2], frame.buffer_mut());
                    } else {
                        let color = battery_level.battery_level_to_color();

                        Line::from(vec![Span::from(format!("{}%", battery_level)).fg(color)])
                            .right_aligned()
                            .render(h1_h[1], frame.buffer_mut());

                        LineGauge::default()
                            .unfilled_symbol("\u{258C}")
                            .unfilled_style(Style::new().dark_gray())
                            .filled_symbol("\u{258C}")
                            .filled_style(Style::new().fg(color))
                            .ratio(battery_level as f64 / 100.0)
                            .label("")
                            .render(h1_h[2], frame.buffer_mut());
                    }
                }

                if let Some(my_node) = state.get_my_node() {
                    short_name_to_span(my_node, true).render(h1_h[4], frame.buffer_mut());
                }

                status_line.render(h1_h[6], frame.buffer_mut());
            }
        };
    }
}
