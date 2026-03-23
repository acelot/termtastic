use crate::ui::prelude::*;

pub struct TerminalSize {
    required_size: (u16, u16),
}

impl TerminalSize {
    pub fn new(required_size: (u16, u16)) -> Self {
        Self { required_size }
    }
}

impl Component for TerminalSize {
    fn handle_event(&mut self, _state: &State, _event: &Event, _emit: &impl Fn(AppEvent)) {}

    fn render(&mut self, _state: &State, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(area);

        let warning = Paragraph::new(
            Span::from(" TERMINAL SIZE IS TOO SMALL! ").style(Style::new().white().on_red()),
        )
        .centered();

        frame.render_widget(warning, v[1]);

        let sizes = Paragraph::new(format!(
            "{}x{} [{}x{}]",
            self.required_size.0, self.required_size.1, area.width, area.height
        ))
        .centered();

        frame.render_widget(sizes, v[3]);
    }
}
