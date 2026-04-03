use chrono::{SubsecRound, TimeDelta, Utc};

use crate::ui::{helpers::ColorExt, prelude::*};

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
                    key: "↑↓".to_string(),
                    label: "scroll".to_string(),
                },
                Hotkey {
                    key: "enter".to_string(),
                    label: "expand".to_string(),
                },
                Hotkey {
                    key: "s".to_string(),
                    label: "sort by".to_string(),
                },
            ]),
        }
    }
}

impl Component for Nodes {
    fn handle_event(
        &mut self,
        state: &State,
        event: &Event,
        _emit: &impl Fn(AppEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Up => self.list_state.previous(),
                KeyCode::Down => self.list_state.next(),
                KeyCode::Home => {
                    self.list_state.select(Some(0));
                }
                KeyCode::End => {
                    self.list_state.select(Some(state.nodes_sort.len() - 1));
                }
                _ => {}
            },
            Event::Mouse(MouseEvent { kind, .. }) => match kind {
                MouseEventKind::ScrollUp => self.list_state.previous(),
                MouseEventKind::ScrollDown => self.list_state.next(),
                _ => {}
            },
            _ => {}
        }

        Ok(())
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        if !state.nodes_sort.is_empty() && self.list_state.selected.is_none() {
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
            let node = &state.nodes[&state.nodes_sort[context.index as usize]];

            let item = NodeWidget {
                node,
                is_selected: context.is_selected,
            };

            (item, 3)
        });

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .symbols(ScrollbarSet {
                begin: "┬",
                thumb: "█",
                track: "│",
                end: "┴",
            })
            .style(Style::new().dark_gray());

        let list = ListView::new(list_builder, state.nodes_sort.len())
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
            height: area.height - 1,
        };

        let block = Block::bordered()
            .borders(Borders::LEFT)
            .border_set(if self.is_selected {
                symbols::border::THICK
            } else {
                symbols::border::PLAIN
            })
            .border_style(Style::new().fg(if self.is_selected {
                Color::Yellow
            } else {
                Color::DarkGray
            }))
            .padding(Padding::symmetric(1, 0));

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
            self.node.to_span(),
            Span::from(" "),
            Span::from(self.node.long_name.clone()),
        ])
        .render(v0_h[0], buf);

        Line::from(match self.node.hops_away {
            Some(0) => Span::from(format!("⁕ {}dB", self.node.snr))
                .style(Style::new().fg(self.node.snr.snr_to_color())),
            Some(hops) => Span::from("❱".repeat(hops as usize)).dark_gray(),
            None if self.node.my => Span::from("✔ connected").blue(),
            None => Span::from("unknown").dark_gray(),
        })
        .render(v0_h[1], buf);

        let last_heard_spans: Vec<Span> = match self.node.last_heard {
            Some(_) if self.node.my => vec![Span::from("now").blue()],
            Some(dt) => humanize_duration(Utc::now().round_subsecs(0) - dt),
            None => vec![Span::from("?").dark_gray()],
        };

        Line::from(last_heard_spans)
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

fn humanize_duration<'a>(d: TimeDelta) -> Vec<Span<'a>> {
    if d.num_seconds() < 60 {
        return vec![Span::from("now").green()];
    }

    if d.num_minutes() < 60 {
        return vec![
            Span::from(format!("{}m", d.num_minutes())),
            Span::from(" ago").dark_gray(),
        ];
    }

    if d.num_hours() < 24 {
        let remaining_minutes = d.num_minutes() % 60;

        return vec![
            Span::from(format!("{}h {}m", d.num_hours(), remaining_minutes)),
            Span::from(" ago").dark_gray(),
        ];
    }

    let remaining_hours = d.num_hours() % 24;

    vec![
        Span::from(format!("{}d {}h", d.num_days(), remaining_hours)),
        Span::from(" ago").dark_gray(),
    ]
}
