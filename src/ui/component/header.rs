use crate::ui::prelude::*;

pub struct Header {}

impl Header {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for Header {
    fn handle_event(
        &mut self,
        _state: &State,
        _event: &crossterm::event::Event,
        _emit: &impl Fn(AppEvent),
    ) {
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        let v = Layout::default()
            .direction(Direction::Horizontal)
            .flex(layout::Flex::SpaceBetween)
            .constraints([Constraint::Fill(1), Constraint::Fill(1)])
            .split(area);

        let app_info = vec![
            Span::from(state.app_name.clone()).magenta().bold(),
            Span::from(" "),
            Span::from(format!("v{}", state.app_version)).dark_gray(),
        ];

        frame.render_widget(Paragraph::new(Line::from(app_info)), v[0]);

        let my_node_info = if let Some(my_node) = state.get_my_node() {
            vec![
                Span::from("node ").dark_gray(),
                my_node.to_span(),
                Span::from("  "),
            ]
        } else {
            vec![]
        };

        let conn_info = match state.connection_state {
            ConnectionState::NotConnected => vec![Span::from("not connected").dark_gray()],
            ConnectionState::ProblemDetected { .. } => {
                vec![
                    Span::from(format!(
                        "reconnecting in {} sec...",
                        state
                            .reconnection_backoff
                            .and_then(|b| Some(b.as_secs().to_string()))
                            .unwrap_or("?".to_owned())
                    ))
                    .red(),
                ]
            }
            ConnectionState::Connecting => vec![Span::from("connecting...").yellow()],
            ConnectionState::Connected => vec![
                Span::from("online").dark_gray(),
                Span::from(" "),
                Span::from(format!("{}/{}", state.online_nodes, state.nodes.len())).green(),
                Span::from("  "),
                if state.rx {
                    Span::from("■").green()
                } else {
                    Span::from("■").dark_gray()
                },
            ],
        };

        frame.render_widget(
            Line::from([my_node_info.as_slice(), conn_info.as_slice()].concat()).right_aligned(),
            v[1],
        );
    }
}
