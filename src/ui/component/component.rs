use crossterm::event::Event;

use crate::ui::prelude::*;

pub trait Component {
    fn get_hotkeys(&self, state: &State) -> Vec<Hotkey>;

    fn handle_event(&mut self, state: &State, event: &Event, emit: &impl Fn(AppEvent)) -> bool;

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect);
}
