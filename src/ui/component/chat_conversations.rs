use crate::ui::prelude::*;

pub struct ChatConversations {
    list_state: ListState,
    hotkeys_component: Hotkeys,
}

impl ChatConversations {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            hotkeys_component: Hotkeys::new(vec![
                Hotkey {
                    key: "enter".to_string(),
                    label: "select".to_string(),
                },
                Hotkey {
                    key: "\u{2191}\u{2193}".to_string(),
                    label: "navigate".to_string(),
                },
            ]),
        }
    }
}

impl Component for ChatConversations {
    fn handle_event(&mut self, _state: &State, event: &Event, emit: &impl Fn(AppEvent)) {
        if let Event::Key(KeyEvent { code, .. }) = event {
            match code {
                KeyCode::Enter => emit(AppEvent::ChatConversationSelected("test".to_string())),
                _ => {}
            };
        }
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        let v = ratatui::layout::Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);

        frame.render_widget(Paragraph::new("Select channel:".to_string()), v[0]);

        let list_items: Vec<ListItem> = state
            .conversations
            .iter()
            .map(|(_, chat)| ListItem::new(Span::from(chat.name.clone())))
            .collect();

        let list = List::new(list_items)
            .direction(ListDirection::TopToBottom)
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol(">".to_string());

        frame.render_stateful_widget(list, v[1], &mut self.list_state);

        self.hotkeys_component.render(state, frame, v[2]);
    }
}
