use crossterm::event::Event;

use crate::ui::prelude::*;

pub trait Component {
    fn handle_event(&mut self, state: &State, event: &Event, emit: &impl Fn(AppEvent));

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect);
}
