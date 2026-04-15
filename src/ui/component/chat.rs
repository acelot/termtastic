use crate::ui::component::{Channels, Messenger};
use crate::ui::prelude::*;

pub struct Chat<'a> {
    messenger_component: Messenger<'a>,
    channels_component: Channels,
}

impl<'a> Chat<'a> {
    pub fn new() -> Self {
        Self {
            messenger_component: Messenger::new(),
            channels_component: Channels::new(),
        }
    }
}

impl<'a> Component for Chat<'a> {
    fn handle_event(
        &mut self,
        state: &State,
        event: &Event,
        emit: &impl Fn(AppEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        if state.active_channel_key.is_some() {
            self.messenger_component.handle_event(state, event, emit)?;
        } else {
            self.channels_component.handle_event(state, event, emit)?;
        }

        Ok(())
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        if state.active_channel_key.is_some() {
            self.messenger_component.render(state, frame, area);
        } else {
            self.channels_component.render(state, frame, area);
        }
    }
}
