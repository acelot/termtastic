use itertools::Itertools;
use tracing_unwrap::OptionExt;

use crate::ui::prelude::*;

pub struct ChatChannels {
    channels: Vec<Channel>,
    list_state: ListState,
    hotkeys_component: Hotkeys,
}

impl ChatChannels {
    pub fn new() -> Self {
        Self {
            channels: vec![],
            list_state: ListState::default(),
            hotkeys_component: Hotkeys::new(vec![
                Hotkey {
                    key: "↑↓".to_string(),
                    label: "navigate".to_string(),
                },
                Hotkey {
                    key: "enter".to_string(),
                    label: "open".to_string(),
                },
            ]),
        }
    }
}

impl Component for ChatChannels {
    fn handle_event(&mut self, state: &State, event: &Event, emit: &impl Fn(AppEvent)) {
        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Up => self.list_state.previous(),
                KeyCode::Down => self.list_state.next(),
                KeyCode::Enter => {
                    if let Some(i) = self.list_state.selected {
                        let channel = self.channels.get(i).unwrap();

                        emit(AppEvent::ChannelSelected(channel.index));
                    }
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
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        self.channels = state
            .channels
            .values()
            .filter(|ch| !ch.role.is_disabled())
            .sorted_by_key(|ch| ch.index)
            .cloned()
            .collect();

        if !self.channels.is_empty() && self.list_state.selected.is_none() {
            self.list_state.select(Some(0));
        }

        let v = ratatui::layout::Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        let list_builder = ListBuilder::new(|context| {
            let channel = self.channels.get(context.index).unwrap();

            let item = ChannelWidget {
                channel,
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

        let list = ListView::new(list_builder, state.channels.len())
            .infinite_scrolling(false)
            .scrollbar(scrollbar);

        list.render(v[0], frame.buffer_mut(), &mut self.list_state);

        self.hotkeys_component.render(state, frame, v[1]);
    }
}

struct ChannelWidget<'a> {
    pub channel: &'a Channel,
    pub is_selected: bool,
}

impl<'a> Widget for ChannelWidget<'a> {
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
            .constraints([Constraint::Min(1)])
            .split(v[0]);

        // first line
        let name_span = match (&self.channel.role, self.channel.name.is_empty()) {
            (ChannelRole::Primary, false) => Span::from(self.channel.name.clone()),
            (ChannelRole::Primary, true) => Span::from("Primary".to_string()),
            (ChannelRole::Secondary, false) => Span::from(self.channel.name.clone()),
            (ChannelRole::Secondary, true) => Span::from(format!("Secondary #{}", self.channel.id)),
            _ => unreachable!(),
        };

        Line::from(name_span)
            .add_modifier(if self.is_selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            })
            .render(v0_h[0], buf);
    }
}
