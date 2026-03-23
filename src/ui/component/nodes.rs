use chrono::{SubsecRound, Utc};

use crate::ui::prelude::*;

pub struct Nodes {
    list_state: ListState,
    hotkeys_component: Hotkeys,
}

impl Nodes {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
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
                    label: "to top".to_string(),
                },
                Hotkey {
                    key: "end".to_string(),
                    label: "to bottom".to_string(),
                },
            ]),
        }
    }
}

impl Component for Nodes {
    fn handle_event(&mut self, _state: &State, event: &Event, _emit: &impl Fn(AppEvent)) {
        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Up => self.list_state.previous(),
                KeyCode::Down => self.list_state.next(),
                _ => {}
            },
            Event::Mouse(MouseEvent { kind, .. }) => match kind {
                MouseEventKind::ScrollUp => self.list_state.previous(),
                MouseEventKind::ScrollDown => self.list_state.next(),
                _ => {}
            },
            _ => {}
        }
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        if !state.nodes.is_empty() && self.list_state.selected.is_none() {
            self.list_state.select(Some(0));
        }

        let v = ratatui::layout::Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        let list_builder = ListBuilder::new(|context| {
            let (_, node) = state.nodes.iter().nth(context.index).unwrap();

            let item = NodeWidget {
                node,
                is_selected: context.is_selected,
            };

            (item, 4)
        });

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .symbols(ScrollbarSet {
                begin: "┬",
                thumb: "█",
                track: "│",
                end: "┴",
            })
            .style(Style::new().dark_gray());

        let list = ListView::new(list_builder, state.nodes.len())
            .infinite_scrolling(false)
            .scrollbar(scrollbar);

        list.render(v[0], frame.buffer_mut(), &mut self.list_state);

        self.hotkeys_component.render(state, frame, v[2]);
    }
}

struct NodeWidget<'a> {
    pub node: &'a Node,
    pub is_selected: bool,
}

impl<'a> Widget for NodeWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let area = Rect {
            x: area.x,
            y: area.y,
            width: area.width - 2,
            height: area.height,
        };

        let mut block = Block::bordered()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().dark_gray())
            .padding(Padding::symmetric(1, 0));

        if self.is_selected {
            block = block.border_style(Style::new().yellow());
        }

        let block_area = block.inner(area);
        block.render(area, buf);

        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(block_area);

        let v0_h = Layout::default()
            .direction(Direction::Horizontal)
            .flex(layout::Flex::SpaceBetween)
            .constraints([
                Constraint::Fill(2),
                Constraint::Fill(1),
                Constraint::Fill(1),
            ])
            .split(v[0]);

        let v1_h = Layout::default()
            .direction(Direction::Horizontal)
            .flex(layout::Flex::SpaceBetween)
            .constraints([
                Constraint::Fill(2),
                Constraint::Fill(1),
                Constraint::Fill(1),
            ])
            .split(v[1]);

        // first line
        Line::from(vec![
            Span::from(format!(" {:<4} ", self.node.short_name))
                .black()
                .on_green(),
            Span::from(" "),
            Span::from(self.node.long_name.clone()),
        ])
        .style(if self.is_selected {
            Style::new().bold()
        } else {
            Style::new()
        })
        .render(v0_h[0], buf);

        Line::from(match self.node.hops_away {
            Some(0) => Span::from(format!("direct {} dB", self.node.snr)).green(),
            Some(hops) => Span::from(format!("hops away: {}", hops.to_string())),
            None => Span::from("hops: ?".to_string()),
        })
        .render(v0_h[1], buf);

        let last_heard = match self.node.last_heard {
            Some(dt) => match (Utc::now().round_subsecs(0) - dt).to_std() {
                Ok(d) => humantime::format_duration(d).to_string(),
                Err(_) => "?".to_owned(),
            },
            None => "?".to_owned(),
        };

        Line::from(vec![
            Span::from(last_heard),
            Span::from(" ago".to_owned()).dark_gray(),
        ])
        .right_aligned()
        .render(v0_h[2], buf);

        // second line
        Line::from(vec![Span::from(self.node.hw_model.clone()).magenta()]).render(v1_h[0], buf);

        Line::from(vec![Span::from(self.node.role.clone()).dark_gray()]).render(v1_h[1], buf);

        Line::from(vec![Span::from(self.node.id.clone()).dark_gray()])
            .right_aligned()
            .render(v1_h[2], buf);
    }
}
