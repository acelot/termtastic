use std::collections::VecDeque;

use chrono::Local;
use itertools::Itertools;

use crate::ui::{helpers::default_scrollbar, prelude::*};

pub struct Channels {
    channels: Vec<Channel>,
    list_state: ListState,
    hotkeys: Vec<Hotkey>,
}

impl Channels {
    pub fn new() -> Self {
        Self {
            channels: vec![],
            list_state: ListState::default(),
            hotkeys: vec![
                Hotkey {
                    key: "↑↓".to_string(),
                    label: "scroll".to_string(),
                },
                Hotkey {
                    key: "enter".to_string(),
                    label: "open".to_string(),
                },
            ],
        }
    }
}

impl<'a> Component for Channels {
    fn handle_event(
        &mut self,
        _state: &State,
        event: &Event,
        emit: &impl Fn(AppEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<bool> {
        match event {
            Event::Key(KeyEvent { code, kind, .. }) if kind == &KeyEventKind::Press => match code {
                KeyCode::Up => self.list_state.previous(),
                KeyCode::Down => self.list_state.next(),
                KeyCode::Enter => {
                    if let Some(i) = self.list_state.selected {
                        let channel = self.channels.get(i).unwrap();

                        emit(AppEvent::ChannelSelected(channel.key))?;
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

        Ok(true)
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        let v = ratatui::layout::Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        self.channels = state
            .channels
            .values()
            .filter(|ch| !ch.role.is_disabled())
            .sorted_by_key(|ch| ch.key)
            .cloned()
            .collect();

        if !self.channels.is_empty() {
            if self.list_state.selected.is_none() {
                self.list_state.select(Some(0));
            }

            let empty_messages_vec: VecDeque<Message> = VecDeque::default();

            let list_builder = ListBuilder::new(|context| {
                let channel = self.channels.get(context.index).unwrap();

                let messages = state
                    .messages
                    .get(&channel.key)
                    .unwrap_or(&empty_messages_vec);

                let last_message = messages.iter().last();
                let last_message_node =
                    last_message.and_then(|message| state.nodes.get(&message.from));

                let item = ConversationWidget {
                    channel,
                    direct_node: if channel.role.is_direct() {
                        state.nodes.get(&channel.key)
                    } else {
                        None
                    },
                    last_message,
                    last_message_node,
                    is_selected: context.is_selected,
                };

                (item, 4)
            });

            let list = ListView::new(list_builder, self.channels.len())
                .infinite_scrolling(false)
                .scrollbar(default_scrollbar());

            list.render(v[0], frame.buffer_mut(), &mut self.list_state);
        } else {
            PlaceholderWidget::black_on_dark_gray(" no channels ").render(v[0], frame.buffer_mut());
        }

        HotkeysWidget::new(&self.hotkeys).render(v[1], frame.buffer_mut());
    }
}

struct ConversationWidget<'a> {
    pub channel: &'a Channel,
    pub direct_node: Option<&'a Node>,
    pub last_message: Option<&'a Message>,
    pub last_message_node: Option<&'a Node>,
    pub is_selected: bool,
}

impl<'a> Widget for ConversationWidget<'a> {
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

        let block = Block::bordered()
            .border_type(if self.is_selected {
                BorderType::Thick
            } else {
                BorderType::Plain
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
            .constraints([
                Constraint::Fill(3),
                Constraint::Fill(1),
                Constraint::Fill(1),
            ])
            .split(v[0]);

        // first line
        let name_span = match (
            &self.channel.role,
            self.channel.name.is_empty(),
            self.direct_node,
        ) {
            (ChannelRole::Primary, false, _) => vec![
                Span::from(format!("#{}", self.channel.key)).dark_gray(),
                Span::from(" "),
                Span::from(self.channel.name.clone()),
            ],
            (ChannelRole::Primary, true, _) => {
                vec![
                    Span::from(format!("#{}", self.channel.key)).dark_gray(),
                    Span::from(" Primary"),
                ]
            }
            (ChannelRole::Secondary, false, _) => vec![
                Span::from(format!("#{}", self.channel.key)).dark_gray(),
                Span::from(" "),
                Span::from(self.channel.name.clone()),
            ],
            (ChannelRole::Secondary, true, _) => {
                vec![
                    Span::from(format!("#{}", self.channel.key)).dark_gray(),
                    Span::from(" Secondary"),
                ]
            }
            (ChannelRole::Direct, true, Some(node)) => {
                vec![
                    Span::from(format!("{:^6}", node.short_name))
                        .black()
                        .on_green(),
                    Span::from(" "),
                    Span::from(node.long_name.clone()),
                ]
            }
            (ChannelRole::Direct, true, None) => {
                vec![Span::from(format!("Direct from {}", self.channel.key))]
            }
            _ => unreachable!(),
        };

        Line::from(name_span).render(v0_h[0], buf);

        let type_span = match &self.channel.role {
            ChannelRole::Primary => Span::from("PRIMARY"),
            ChannelRole::Secondary => Span::from("SECONDARY"),
            ChannelRole::Direct => Span::from("DIRECT"),
            _ => unreachable!(),
        };

        Line::from(type_span).magenta().render(v0_h[1], buf);

        Line::from(if let Some(message) = self.last_message {
            Span::from(
                message
                    .datetime
                    .with_timezone(&Local)
                    .format("%H:%M")
                    .to_string(),
            )
        } else {
            Span::from("no messages").dark_gray()
        })
        .right_aligned()
        .render(v0_h[2], buf);

        // second line
        let unknown_node = &Node::unknown();

        let second_line_spans = match (
            &self.channel.role,
            self.last_message_node,
            self.last_message,
        ) {
            (ChannelRole::Direct, _, Some(message)) => {
                vec![Span::from(message.text.clone()).dark_gray()]
            }
            (_, None, Some(message)) => {
                vec![
                    unknown_node.to_span(),
                    Span::from(" "),
                    Span::from(message.text.clone()).dark_gray(),
                ]
            }
            (_, Some(node), Some(message)) => {
                vec![
                    node.to_span(),
                    Span::from(" "),
                    Span::from(message.text.clone()).dark_gray(),
                ]
            }
            (_, _, None) => {
                vec![]
            }
        };

        Line::from(second_line_spans).render(v[1], buf);
    }
}
