use std::{
    collections::{HashMap, VecDeque},
    iter,
    ops::RangeInclusive,
    sync::LazyLock,
};

use crossterm::event::KeyModifiers;
use itertools::Itertools;
use ratatui::text::ToSpan;
use tracing_unwrap::OptionExt;
use tui_widget_list::ScrollDirection;

use crate::ui::{
    helpers::{ColorExt, default_scrollbar},
    prelude::*,
};

const INPUT_VALUE_MAX_LENGTH: usize = 200;
const VALID_INPUT_LENGTH: RangeInclusive<usize> = 1..=INPUT_VALUE_MAX_LENGTH;

static UNKNOWN_NODE: LazyLock<Node> = LazyLock::new(|| Node::unknown());
static EMPTY_MESSAGES_VEC: LazyLock<VecDeque<Message>> = LazyLock::new(|| VecDeque::default());

pub struct Messenger<'a> {
    list_states: HashMap<u32, ListState>,
    input_widgets: HashMap<u32, TextArea<'a>>,
    follow_chat: HashMap<u32, bool>,
    replying_to: HashMap<u32, (Node, u32)>,
}

impl<'a> Messenger<'a> {
    pub fn new() -> Self {
        Self {
            list_states: HashMap::default(),
            input_widgets: HashMap::default(),
            follow_chat: HashMap::default(),
            replying_to: HashMap::default(),
        }
    }

    fn get_hotkeys(&self, active_channel_key: u32) -> Vec<Hotkey> {
        let is_message_selected = self
            .list_states
            .get(&active_channel_key)
            .and_then(|s| Some(s.selected.is_some()))
            .unwrap_or(false);

        let has_input_value = self
            .input_widgets
            .get(&active_channel_key)
            .and_then(|input| Some(VALID_INPUT_LENGTH.contains(&input.lines()[0].len())))
            .unwrap_or(false);

        let is_replying_to = self.replying_to.contains_key(&active_channel_key);

        vec![
            Some(Hotkey::new("↑↓", "scroll")),
            (is_message_selected && !is_replying_to).then_some(Hotkey::new("F2", "reply")),
            (is_message_selected && !is_replying_to).then_some(Hotkey::new("F4", "node info")),
            (!is_replying_to).then_some(Hotkey::new("esc", "switch channel")),
            is_replying_to.then_some(Hotkey::new("esc", "cancel reply")),
            has_input_value.then_some(Hotkey::new(
                "enter",
                if is_replying_to { "send reply" } else { "send" },
            )),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

impl<'a> Component for Messenger<'a> {
    fn handle_event(
        &mut self,
        state: &State,
        event: &Event,
        emit: &impl Fn(AppEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let active_channel_key = state
            .active_channel_key
            .expect_or_log("channel should be selected");

        let list_state = self
            .list_states
            .entry(active_channel_key)
            .or_insert_with(|| ListState::default());

        let input_widget = self
            .input_widgets
            .entry(active_channel_key)
            .or_insert_with(|| new_input_widget());

        let is_replying_to = self.replying_to.contains_key(&active_channel_key);

        let messages = state
            .messages
            .get(&active_channel_key)
            .unwrap_or(&EMPTY_MESSAGES_VEC);

        match event {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => match code {
                KeyCode::Up => {
                    self.follow_chat.insert(active_channel_key, false);
                    list_state.previous()
                }
                KeyCode::Down => {
                    list_state.next();

                    if let Some(index) = list_state.selected {
                        self.follow_chat
                            .insert(active_channel_key, index == messages.len() - 1);
                    }
                }
                KeyCode::Esc if !is_replying_to => emit(AppEvent::SwitchChannelRequested)?,
                KeyCode::Esc if is_replying_to => {
                    self.replying_to.remove(&active_channel_key);
                }
                KeyCode::Enter if modifiers.contains(KeyModifiers::CONTROL) => {
                    input_widget.insert_newline();
                }
                KeyCode::Enter if !is_replying_to => {
                    if input_widget.lines()[0].len() <= INPUT_VALUE_MAX_LENGTH {
                        emit(AppEvent::ChatMessageSubmitted {
                            text: input_widget.lines()[0].clone(),
                            reply_message_id: None,
                        })?;

                        input_widget.clear();
                    }
                }
                KeyCode::Enter if is_replying_to => {
                    if input_widget.lines()[0].len() <= INPUT_VALUE_MAX_LENGTH
                        && let Some((_, message_id)) = self.replying_to.remove(&active_channel_key)
                    {
                        emit(AppEvent::ChatMessageSubmitted {
                            text: input_widget.lines()[0].clone(),
                            reply_message_id: Some(message_id),
                        })?;

                        input_widget.clear();
                    }
                }
                KeyCode::F(2) => {
                    if let Some(index) = list_state.selected
                        && let Some(message) = messages.get(index)
                    {
                        let node = state.nodes.get(&message.from).unwrap_or(&UNKNOWN_NODE);
                        self.replying_to
                            .insert(active_channel_key, (node.clone(), message.id));
                    }
                }
                _ => {
                    input_widget.input(event.clone());
                }
            },
            Event::Mouse(MouseEvent { kind, .. }) => match kind {
                MouseEventKind::ScrollUp => {
                    self.follow_chat.insert(active_channel_key, false);
                    list_state.previous();
                }
                MouseEventKind::ScrollDown => {
                    list_state.next();

                    if let Some(index) = list_state.selected {
                        self.follow_chat
                            .insert(active_channel_key, index == messages.len() - 1);
                    }
                }
                _ => {}
            },
            _ => {}
        }

        Ok(())
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        let active_channel = state
            .get_active_channel()
            .expect_or_log("channel should be selected");

        let list_state = self
            .list_states
            .entry(active_channel.key)
            .or_insert_with(|| ListState::default());

        let input_widget = self
            .input_widgets
            .entry(active_channel.key)
            .or_insert_with(|| new_input_widget());

        let replying_to = self.replying_to.get(&active_channel.key);

        let messages = state
            .messages
            .get(&active_channel.key)
            .unwrap_or(&EMPTY_MESSAGES_VEC);

        let follow_chat = self.follow_chat.entry(active_channel.key).or_insert(true);
        if *follow_chat && !messages.is_empty() {
            list_state.select(Some(messages.len() - 1));
        }

        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(area);

        // list
        if !messages.is_empty() {
            let list_builder = ListBuilder::new(|context| {
                let message = &messages[context.index as usize];
                let replied_message = if message.reply_message_id > 0 {
                    messages
                        .iter()
                        .find(|m| m.id == message.reply_message_id)
                        .and_then(|m| Some((state.nodes.get(&m.from).unwrap_or(&UNKNOWN_NODE), m)))
                } else {
                    None
                };
                let node = state.nodes.get(&message.from).unwrap_or(&UNKNOWN_NODE);

                let item = MessageWidget {
                    node: &node,
                    message,
                    replied_message,
                    is_selected: context.is_selected,
                    is_highlighted: replying_to
                        .and_then(|(_, msg_key)| Some(message.id == *msg_key))
                        .unwrap_or(false),
                };

                let mut height = item.height(area.width);

                if context.index < messages.len() - 1 {
                    height += 1;
                }

                (item, height)
            });

            let list = ListView::new(list_builder, messages.len())
                .infinite_scrolling(false)
                .scroll_direction(ScrollDirection::Backward)
                .scrollbar(default_scrollbar());

            list.render(v[0], frame.buffer_mut(), list_state);
        } else {
            PlaceholderWidget::dark_gray("no messages").render(v[0], frame.buffer_mut());
        }

        // input
        let input_block = Block::bordered()
            .padding(Padding::symmetric(1, 0))
            .border_type(BorderType::Rounded)
            .border_style(Style::new().dark_gray());

        let input_block_area = input_block.inner(v[1]);

        let channel_name_spans = match (&active_channel.role, replying_to) {
            (ChannelRole::Primary | ChannelRole::Secondary, None) => vec![
                Span::from(format!("#{} ", active_channel.key)).dark_gray(),
                Span::from(if !active_channel.name.is_empty() {
                    &active_channel.name
                } else if active_channel.role == ChannelRole::Primary {
                    "Primary"
                } else {
                    "Secondary"
                })
                .yellow(),
                Span::from(" ←").dark_gray(),
            ],
            (ChannelRole::Direct, None) => vec![
                state
                    .nodes
                    .get(&active_channel.key)
                    .unwrap_or(&UNKNOWN_NODE)
                    .to_span(),
                Span::from(" ←").dark_gray(),
            ],
            (_, Some((node, _))) => vec![
                Span::from("reply to ").cyan(),
                node.to_span(),
                Span::from(" ←").dark_gray(),
            ],
            _ => unreachable!(),
        };

        let channel_line = Line::from(channel_name_spans);

        let input_block_area_h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(channel_line.width() as u16),
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(8),
            ])
            .split(input_block_area);

        input_block.render(v[1], frame.buffer_mut());
        channel_line.render(input_block_area_h[0], frame.buffer_mut());
        frame.render_widget(&*input_widget, input_block_area_h[2]);

        let input_value_len = input_widget.lines()[0].len();

        Line::from(
            Span::from(format!(" {}/{}", input_value_len, INPUT_VALUE_MAX_LENGTH)).style(
                Style::new().fg(if input_value_len > INPUT_VALUE_MAX_LENGTH {
                    Color::Red
                } else {
                    Color::DarkGray
                }),
            ),
        )
        .right_aligned()
        .render(input_block_area_h[3], frame.buffer_mut());

        HotkeysWidget::new(&self.get_hotkeys(active_channel.key)).render(v[2], frame.buffer_mut());
    }
}

fn new_input_widget() -> TextArea<'static> {
    let mut input = TextArea::default();
    input.set_placeholder_text("type message...");
    input.set_cursor_line_style(Style::default());

    input
}

struct MessageWidget<'a> {
    pub node: &'a Node,
    pub message: &'a Message,
    pub replied_message: Option<(&'a Node, &'a Message)>,
    pub is_selected: bool,
    pub is_highlighted: bool,
}

#[allow(unstable_name_collisions)]
impl MessageWidget<'_> {
    pub fn get_text_paragraph(&self) -> Paragraph<'_> {
        let reply_line = self.replied_message.and_then(|(_, m)| {
            let spans: Vec<Span<'_>> = m
                .text
                .split('\n')
                .map(|line| Span::from(line))
                .intersperse(Span::from(" "))
                .collect();

            Some(
                Line::from(
                    iter::once("“".to_span())
                        .chain(spans)
                        .chain(iter::once("”".to_span()))
                        .collect::<Vec<Span<'_>>>(),
                )
                .magenta(),
            )
        });

        let text_lines: Vec<Line<'_>> = self.message.text.split('\n').map(Line::from).collect();

        Paragraph::new(
            reply_line
                .into_iter()
                .chain(text_lines)
                .collect::<Vec<Line<'_>>>(),
        )
        .wrap(Wrap { trim: false })
    }

    pub fn height(&self, width: u16) -> u16 {
        1 + self.get_text_paragraph().line_count(width - 2) as u16
            + !self.message.reactions.is_empty() as u16
    }
}

impl<'a> Widget for MessageWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let text_paragraph = self.get_text_paragraph();
        let text_height = text_paragraph.line_count(area.width - 2) as u16;

        let area = Rect {
            x: area.x,
            y: area.y,
            width: area.width - 2,
            height: 1 + text_height + !self.message.reactions.is_empty() as u16,
        };

        let block = Block::bordered()
            .borders(Borders::LEFT)
            .border_set(if self.is_selected {
                symbols::border::THICK
            } else {
                symbols::border::PLAIN
            })
            .border_style(Style::new().fg(if self.is_highlighted {
                Color::Cyan
            } else if self.is_selected {
                Color::Yellow
            } else {
                Color::DarkGray
            }))
            .padding(Padding::symmetric(1, 0));

        let block_area = block.inner(area);
        block.render(area, buf);

        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints(if self.message.reactions.is_empty() {
                vec![Constraint::Length(1), Constraint::Length(text_height)]
            } else {
                vec![
                    Constraint::Length(1),
                    Constraint::Length(text_height),
                    Constraint::Length(1),
                ]
            })
            .split(block_area);

        // first line
        let v0_h = Layout::default()
            .direction(Direction::Horizontal)
            .flex(layout::Flex::SpaceBetween)
            .constraints([
                Constraint::Fill(4),
                Constraint::Fill(2),
                Constraint::Fill(1),
            ])
            .split(v[0]);

        if let Some((rep_node, _)) = self.replied_message {
            Line::from(vec![
                self.node.to_span(),
                " → ".to_span().dark_gray(),
                rep_node.to_span().on_magenta(),
            ])
            .render(v0_h[0], buf);
        } else {
            Line::from(vec![
                self.node.to_span(),
                " ".to_span(),
                self.node.long_name.clone().to_span().bold(),
            ])
            .render(v0_h[0], buf);
        }

        if !self.node.my {
            if let Some(hops) = self.node.hops_away
                && hops > 0
            {
                Span::from("❱".repeat(hops as usize))
                    .dark_gray()
                    .render(v0_h[1], buf);
            } else {
                Line::from(vec![
                    Span::from(format!("⁕ {}dB", self.message.snr))
                        .fg(self.message.snr.snr_to_color()),
                    Span::from("  ").dark_gray(),
                    Span::from(format!("RSSI {}dBm", self.message.rssi)).dark_gray(),
                ])
                .dark_gray()
                .render(v0_h[1], buf);
            }
        } else {
            if self.message.acked {
                Span::from("✔").green().render(v0_h[1], buf);
            } else {
                Span::from("sent").dark_gray().render(v0_h[1], buf);
            }
        }

        Line::from(
            Span::from(
                self.message
                    .datetime
                    .with_timezone(&chrono::Local)
                    .format("%H:%M")
                    .to_string(),
            )
            .dark_gray(),
        )
        .right_aligned()
        .render(v0_h[2], buf);

        // second line
        text_paragraph.render(v[1], buf);

        // third line
        if !self.message.reactions.is_empty() {
            Line::from(
                self.message
                    .reactions
                    .iter()
                    .map(|(emoji, nodes)| {
                        if nodes.len() > 1 {
                            vec![
                                emoji.to_span(),
                                Span::from(format!("'{}", nodes.len())).dark_gray(),
                                " ".to_span(),
                            ]
                        } else {
                            vec![emoji.to_span(), " ".to_span()]
                        }
                    })
                    .flatten()
                    .collect::<Vec<Span>>(),
            )
            .render(v[2], buf);
        }
    }
}
