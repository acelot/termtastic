use crate::ui::prelude::*;

pub struct Nodes {
    list_state: tui_widget_list::ListState,
    hotkeys_component: Hotkeys,
}

impl Nodes {
    pub fn new() -> Self {
        Self {
            list_state: tui_widget_list::ListState::default(),
            hotkeys_component: Hotkeys::new(vec![
                Hotkey {
                    key: "\u{2191}\u{2193}".to_string(),
                    label: "navigate".to_string(),
                },
                Hotkey {
                    key: "enter".to_string(),
                    label: "expand".to_string(),
                },
                Hotkey {
                    key: "c".to_string(),
                    label: "copy".to_string(),
                },
                Hotkey {
                    key: "home".to_string(),
                    label: "go first".to_string(),
                },
                Hotkey {
                    key: "end".to_string(),
                    label: "go last".to_string(),
                },
            ]),
        }
    }
}

impl Component for Nodes {
    fn handle_event(&mut self, state: &State, event: &Event, _emit: &impl Fn(AppEvent)) {
        if let Event::Key(KeyEvent { code, .. }) = event {
            match code {
                _ => {}
            };
        }
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        let v = ratatui::layout::Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        let list_builder = tui_widget_list::ListBuilder::new(|context| {
            let item = NodeWidget {
                short_name: "BHOP".to_owned(),
                is_selected: context.is_selected,
            };

            (item, 4)
        });

        let list = tui_widget_list::ListView::new(list_builder, state.logs.len());

        list.render(v[0], frame.buffer_mut(), &mut self.list_state);

        self.hotkeys_component.render(state, frame, v[2]);
    }
}

struct NodeWidget {
    pub short_name: String,
    pub is_selected: bool,
}

impl Widget for NodeWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let mut block = Block::bordered()
            .border_type(BorderType::Rounded)
            .padding(Padding::uniform(1));

        if self.is_selected {
            block = block.border_style(Style::new().green());
        }

        let v = ratatui::layout::Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(block.inner(area));

        let v0_h = ratatui::layout::Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(5)])
            .split(v[0]);

        Line::from(Span::from(self.short_name).white().on_green()).render(v0_h[0], buf);

        block.render(area, buf);
    }
}
