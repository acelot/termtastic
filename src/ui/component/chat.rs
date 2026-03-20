use crate::ui::component::{ChatConversations, ChatMessages};
use crate::ui::prelude::*;

pub struct Chat {
    messages_component: ChatMessages,
    channels_component: ChatConversations,
}

impl Chat {
    pub fn new() -> Self {
        Self {
            messages_component: ChatMessages::new(),
            channels_component: ChatConversations::new(),
        }
    }
}

impl Component for Chat {
    fn handle_event(&mut self, state: &State, event: &Event, emit: &impl Fn(AppEvent)) {
        if state.active_chat.is_some() {
            self.messages_component.handle_event(state, event, emit);
        } else {
            self.channels_component.handle_event(state, event, emit);
        }
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        if state.active_chat.is_some() {
            self.messages_component.render(state, frame, area);
        } else {
            self.channels_component.render(state, frame, area);
        }
    }
}
