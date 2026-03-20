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
        if let Event::Key(KeyEvent { code, .. }) = event {
            match code {
                KeyCode::Tab => emit(AppEvent::NextTabRequested),
                KeyCode::BackTab => emit(AppEvent::PreviousTabRequested),
                _ => {}
            }
        }
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        let spans: Vec<Span> = Tab::iter()
            .map(|t| {
                let mut tab = Span::from(t.to_string().to_lowercase());

                if t == state.active_tab {
                    tab = tab.style(Style::new().bold().yellow().underlined());
                }

                tab
            })
            .intersperse(Span::from("  ".to_string()))
            .collect();

        frame.render_widget(Line::from(spans), area);
    }
}
