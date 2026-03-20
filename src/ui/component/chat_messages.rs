use crate::ui::{component::ChatInput, prelude::*};

pub struct ChatMessages {
    list_state: ListState,
    input_component: ChatInput,
    hotkeys_component: Hotkeys,
}

impl ChatMessages {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            input_component: ChatInput::new("type message...".to_string(), 200),
            hotkeys_component: Hotkeys::new(vec![
                Hotkey {
                    key: "enter".to_string(),
                    label: "send message".to_string(),
                },
                Hotkey {
                    key: "\u{2191}\u{2193}".to_string(),
                    label: "scroll".to_string(),
                },
                Hotkey {
                    key: "esc".to_string(),
                    label: "switch channel".to_string(),
                },
            ]),
        }
    }
}

impl Component for ChatMessages {
    fn handle_event(&mut self, state: &State, event: &Event, emit: &impl Fn(AppEvent)) {
        self.input_component.handle_event(state, event, emit);
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        let v = ratatui::layout::Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        let active_chat = state.active_chat.as_ref().unwrap().clone();

        let list_items: Vec<ListItem> = state.conversations[&active_chat]
            .messages
            .iter()
            .map(|m| ListItem::new(Span::from(m.content.clone())))
            .collect();

        let list = List::new(list_items)
            .direction(ListDirection::BottomToTop)
            .highlight_style(Style::default().bg(Color::DarkGray));

        frame.render_stateful_widget(list, v[0], &mut self.list_state);

        self.input_component.render(state, frame, v[1]);
        self.hotkeys_component.render(state, frame, v[2]);
    }
}
