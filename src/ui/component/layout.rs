use crate::ui::{
    component::{Chat, Connection, Header, Logs, Nodes, Settings, Tabs, TerminalSize},
    logo::APP_LOGO_TEXT,
    prelude::*,
};

const MIN_TERMINAL_SIZE: (u16, u16) = (80, 24);

pub struct Layout<'a> {
    terminal_size_component: TerminalSize,
    header_component: Header,
    tabs_component: Tabs,
    chat_component: Chat<'a>,
    nodes_component: Nodes,
    settings_component: Settings<'a>,
    connection_component: Connection<'a>,
    logs_component: Logs,
    logo: Text<'static>,
}

impl<'a> Layout<'a> {
    pub fn new() -> Self {
        Self {
            terminal_size_component: TerminalSize::new(MIN_TERMINAL_SIZE),
            header_component: Header::new(),
            tabs_component: Tabs::new(),
            chat_component: Chat::new(),
            nodes_component: Nodes::new(),
            settings_component: Settings::new(),
            connection_component: Connection::new(),
            logs_component: Logs::new(),
            logo: APP_LOGO_TEXT.clone(),
        }
    }
}

impl<'a> Component for Layout<'a> {
    fn handle_event(
        &mut self,
        state: &State,
        event: &Event,
        emit: &impl Fn(AppEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<bool> {
        if let Event::Key(KeyEvent {
            code: KeyCode::F(12),
            ..
        }) = event
        {
            emit(AppEvent::SplashLogoRequested)?;
        }

        if self.header_component.handle_event(state, event, emit)? {
            return Ok(true)
        }

        if self.tabs_component.handle_event(state, event, emit)? {
            return Ok(true)
        }

        match state.active_tab {
            Tab::Chat => self.chat_component.handle_event(state, event, emit),
            Tab::Nodes => self.nodes_component.handle_event(state, event, emit),
            Tab::Settings => self.settings_component.handle_event(state, event, emit),
            Tab::Connection => self.connection_component.handle_event(state, event, emit),
            Tab::Logs => self.logs_component.handle_event(state, event, emit),
        }
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
            Tab::Settings => self.settings_component.render(state, frame, v[3]),
            Tab::Connection => self.connection_component.render(state, frame, v[3]),
            Tab::Logs => self.logs_component.render(state, frame, v[3]),
        }

        if state.splash_logo {
            let logo_width = self.logo.width() as u16;
            let logo_height = self.logo.height() as u16;

            let logo_popup_area = Rect {
                x: area.x + area.width / 2 - logo_width / 2,
                y: area.y + area.height / 2 - logo_height / 2,
                width: logo_width as u16,
                height: logo_height as u16,
            };

            (&self.logo).render(logo_popup_area, frame.buffer_mut());
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
