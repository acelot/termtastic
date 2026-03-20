use crate::ui::component::{DeviceList, DeviceSelected};
use crate::ui::prelude::*;

pub struct Connection {
    connection_list_component: DeviceList,
    connection_selected_component: DeviceSelected,
}

impl Connection {
    pub fn new() -> Self {
        Self {
            connection_list_component: DeviceList::new(),
            connection_selected_component: DeviceSelected::new(),
        }
    }
}

impl Component for Connection {
    fn handle_event(&mut self, state: &State, event: &Event, emit: &impl Fn(AppEvent)) {
        if state.app_config.selected_device.is_none() {
            self.connection_list_component
                .handle_event(state, event, emit);
        } else {
            self.connection_selected_component
                .handle_event(state, event, emit);
        }
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        if state.app_config.selected_device.is_none() {
            self.connection_list_component.render(state, frame, area);
        } else {
            self.connection_selected_component
                .render(state, frame, area);
        }
    }
}
