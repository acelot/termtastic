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
    fn handle_event(&mut self, state: &State, event: &Event, emit: &impl Fn(AppEvent)) {
        self.header_component.handle_event(state, event, emit);
        self.tabs_component.handle_event(state, event, emit);

        match state.active_tab {
            Tab::Chat => self.chat_component.handle_event(state, event, emit),
            Tab::Nodes => self.nodes_component.handle_event(state, event, emit),
            Tab::Connection => self.connection_component.handle_event(state, event, emit),
            Tab::Logs => self.logs_component.handle_event(state, event, emit),
            _ => {}
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
            Tab::Connection => self.connection_component.render(state, frame, v[3]),
            Tab::Logs => self.logs_component.render(state, frame, v[3]),
            _ => {}
        }
    }
}
