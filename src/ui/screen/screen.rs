use crossterm::event::Event;

use crate::ui::prelude::*;

pub trait Screen {
    fn get_hotkeys(&self, state: &State) -> Vec<Hotkey>;

    fn handle_event(&mut self, state: &State, event: &Event, emit: &impl Fn(UiEvent)) -> bool;

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect);
}
