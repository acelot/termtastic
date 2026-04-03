use crate::ui::{
    component::{Chat, Connection, Header, Logs, Nodes, Tabs, TerminalSize},
    prelude::*,
};

const MIN_TERMINAL_SIZE: (u16, u16) = (80, 24);

pub struct Layout {
    terminal_size_component: TerminalSize,
    header_component: Header,
    tabs_component: Tabs,
    chat_component: Chat,
    nodes_component: Nodes,
    connection_component: Connection,
    logs_component: Logs,
}

impl Layout {
    pub fn new() -> Self {
        Self {
            terminal_size_component: TerminalSize::new(MIN_TERMINAL_SIZE),
            header_component: Header::new(),
            tabs_component: Tabs::new(),
            chat_component: Chat::new(),
            nodes_component: Nodes::new(),
            connection_component: Connection::new(),
            logs_component: Logs::new(),
        }
    }
}

impl Component for Layout {
    fn handle_event(
        &mut self,
        state: &State,
        event: &Event,
        emit: &impl Fn(AppEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        self.header_component.handle_event(state, event, emit)?;
        self.tabs_component.handle_event(state, event, emit)?;

        match state.active_tab {
            Tab::Chat => self.chat_component.handle_event(state, event, emit)?,
            Tab::Nodes => self.nodes_component.handle_event(state, event, emit)?,
            Tab::Connection => self.connection_component.handle_event(state, event, emit)?,
            Tab::Logs => self.logs_component.handle_event(state, event, emit)?,
            _ => {}
        }

        Ok(())
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        if area.width < MIN_TERMINAL_SIZE.0 || area.height < MIN_TERMINAL_SIZE.1 {
            self.terminal_size_component.render(state, frame, area);
            return;
        }

        let container = Block::default().padding(Padding::symmetric(2, 1));
        let area = container.inner(frame.area());

        let v = ratatui::layout::Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(area);

        self.header_component.render(state, frame, v[0]);
        self.tabs_component.render(state, frame, v[1]);

        match state.active_tab {
            Tab::Chat => self.chat_component.render(state, frame, v[3]),
            Tab::Nodes => self.nodes_component.render(state, frame, v[3]),
            Tab::Connection => self.connection_component.render(state, frame, v[3]),
            Tab::Logs => self.logs_component.render(state, frame, v[3]),
            _ => {}
        }

        if let Some(Toast { kind, text, .. }) = &state.toast {
            let toast_width = text.len() as u16 + 4;

            let toast_area = Rect {
                x: area.x + area.width / 2 - toast_width / 2,
                y: area.y + area.height - area.height / 6,
                width: toast_width,
                height: 3,
            };

            let (border_color, text_color) = match kind {
                ToastKind::Success => (Color::Green, Color::Green),
                ToastKind::Normal => (Color::DarkGray, Color::White),
                ToastKind::Warning => (Color::DarkGray, Color::Yellow),
                ToastKind::Error => (Color::Red, Color::Red),
            };

            let toast_block = Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .border_style(Style::new().fg(border_color))
                .padding(Padding::symmetric(1, 0));

            let toast_block_area = toast_block.inner(toast_area);

            Clear.render(toast_area, frame.buffer_mut());
            toast_block.render(toast_area, frame.buffer_mut());

            Paragraph::new(Span::from(text))
                .fg(text_color)
                .centered()
                .render(toast_block_area, frame.buffer_mut());
        }
    }
}
