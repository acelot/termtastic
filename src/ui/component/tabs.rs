use itertools::Itertools;
use strum::IntoEnumIterator;

use crate::ui::prelude::*;

pub struct Tabs {}

impl Tabs {
    pub fn new() -> Self {
        Self {}
    }
}

#[allow(unstable_name_collisions)]
impl Component for Tabs {
    fn handle_event(&mut self, _state: &State, event: &Event, emit: &impl Fn(AppEvent)) {
        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Tab => emit(AppEvent::NextTabRequested),
                KeyCode::BackTab => emit(AppEvent::PreviousTabRequested),
                _ => {}
            },
            _ => {}
        }
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        let spans: Vec<Span> = Tab::iter()
            .map(|tab| {
                let mut span = Span::from(tab.to_string().to_lowercase());

                if tab == state.active_tab {
                    span = span.style(Style::new().bold().yellow().underlined());
                }

                span
            })
            .intersperse(Span::from("  ".to_string()))
            .collect();

        frame.render_widget(Line::from(spans), area);
    }
}
