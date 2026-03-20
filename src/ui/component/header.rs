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
        let app_info_length = state.app_name.len() + state.app_version.len() + 1;

        let v = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(app_info_length as u16),
                Constraint::Min(1),
            ])
            .split(area);

        let app_info = vec![
            Span::from(state.app_name.clone()).magenta().bold(),
            Span::from(" "),
            Span::from(state.app_version.clone()).dark_gray(),
        ];

        frame.render_widget(Paragraph::new(Line::from(app_info)), v[0]);

        let conn_info = match state.connection_state {
            ConnectionState::NotConnected => vec![Span::from("not connected").dark_gray()],
            ConnectionState::ProblemDetected { since, .. } => {
                vec![Span::from(format!("on pause {} secs", since.elapsed().as_secs())).red()]
            }
            ConnectionState::Connecting => vec![Span::from("connecting...").yellow()],
            ConnectionState::Connected => vec![
                Span::from("online"),
                Span::from(" "),
                Span::from("10/200").yellow(),
                Span::from("  "),
                Span::from("rx "),
                Span::from("■").green(),
                Span::from(" "),
                Span::from("tx "),
                Span::from("■").red(),
            ],
        };

        frame.render_widget(Line::from(conn_info).right_aligned(), v[1]);
    }
}
