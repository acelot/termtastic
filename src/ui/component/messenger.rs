use std::collections::{HashMap, VecDeque};

use ratatui::text::ToSpan;
use tracing_unwrap::OptionExt;
use tui_input::backend::crossterm::EventHandler;
use tui_widget_list::ScrollDirection;

use crate::ui::prelude::*;

const INPUT_VALUE_MAX_LENGTH: usize = 200;

pub struct Messenger {
    list_states: HashMap<u32, ListState>,
    input_widgets: HashMap<u32, TuiInput>,
    follow_chat: HashMap<u32, bool>,
}

impl Messenger {
    pub fn new() -> Self {
        Self {
            list_states: HashMap::default(),
            input_widgets: HashMap::default(),
            follow_chat: HashMap::default(),
        }
    }

    fn get_hotkeys(&self, state: &State) -> Vec<Hotkey> {
        let active_channel_key = state.active_channel_key.unwrap_or_log();

        let is_message_selected = self
            .list_states
            .get(&active_channel_key)
            .and_then(|s| Some(s.selected.is_some()))
            .unwrap_or(false);

        let has_input_value = self
            .input_widgets
            .get(&active_channel_key)
            .and_then(|input| Some(input.value().len() > 0))
            .unwrap_or(false);

        vec![
            Some(Hotkey {
                key: "↑↓".to_string(),
                label: "scroll".to_string(),
            }),
            if is_message_selected {
                Some(Hotkey {
                    key: "F2".to_string(),
                    label: "reply".to_string(),
                })
            } else {
                None
            },
            Some(Hotkey {
                key: "esc".to_string(),
                label: "switch channel".to_string(),
            }),
            if has_input_value {
                Some(Hotkey {
                    key: "enter".to_string(),
                    label: "send".to_string(),
                })
            } else {
                None
            },
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

impl Component for Messenger {
    fn handle_event(&mut self, state: &State, event: &Event, emit: &impl Fn(AppEvent)) {
        let active_channel_key = state.active_channel_key.unwrap_or_log();

        let list_state = self
            .list_states
            .entry(active_channel_key)
            .or_insert(ListState::default());

        let input_widget = self
            .input_widgets
            .entry(active_channel_key)
            .or_insert(TuiInput::default());

        let messages_len = state
            .messages
            .get(&active_channel_key)
            .and_then(|m| Some(m.len()))
            .unwrap_or(0);

        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Up => {
                    self.follow_chat.insert(active_channel_key, false);
                    list_state.previous()
                }
                KeyCode::Down => {
                    list_state.next();

                    if let Some(index) = list_state.selected {
                        self.follow_chat
                            .insert(active_channel_key, index == messages_len - 1);
                    }
                }
                KeyCode::Esc => emit(AppEvent::SwitchChannelRequested),
                KeyCode::Enter => {
                    if input_widget.value().len() <= INPUT_VALUE_MAX_LENGTH {
                        let text = input_widget.value_and_reset();
                        emit(AppEvent::ChatMessageSubmitted(text));
                    }
                }
                _ => {
                    input_widget.handle_event(event);
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
                            .insert(active_channel_key, index == messages_len - 1);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        let active_channel_key = state.active_channel_key.unwrap_or_log();
        let unknown_node = Node::unknown();
        let empty_messages_vec: VecDeque<Message> = VecDeque::default();

        let list_state = self
            .list_states
            .entry(active_channel_key)
            .or_insert(ListState::default());

        let input_widget = self
            .input_widgets
            .entry(active_channel_key)
            .or_insert(TuiInput::default());

        let messages = state
            .messages
            .get(&active_channel_key)
            .unwrap_or(&empty_messages_vec);

        let follow_chat = self.follow_chat.entry(active_channel_key).or_insert(true);
        if *follow_chat && !messages.is_empty() {
            list_state.select(Some(messages.len() - 1));
        }

        // list
        let list_builder = ListBuilder::new(|context| {
            let message = &messages[context.index as usize];
            let node = state.nodes.get(&message.from).unwrap_or(&unknown_node);

            let item = MessageWidget {
                node: &node,
                message,
                is_selected: context.is_selected,
            };

            let mut height = item.height(area.width);

            if context.index < messages.len() - 1 {
                height += 1;
            }

            (item, height)
        });

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .symbols(ScrollbarSet {
                begin: "┬",
                thumb: "█",
                track: "│",
                end: "┴",
            })
            .style(Style::new().dark_gray());

        let list = ListView::new(list_builder, messages.len())
            .infinite_scrolling(false)
            .scroll_direction(ScrollDirection::Backward)
            .scrollbar(scrollbar);

        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(area);

        list.render(v[0], frame.buffer_mut(), list_state);

        // input
        let input_block = Block::bordered()
            .padding(Padding::symmetric(1, 0))
            .border_type(BorderType::Rounded)
            .border_style(Style::new().dark_gray());

        let input_block_area = input_block.inner(v[1]);

        let input_block_area_h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(6),
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(8),
            ])
            .split(input_block_area);

        input_block.render(v[1], frame.buffer_mut());

        let my_node = state.get_my_node().unwrap_or(&unknown_node);

        Line::from(
            Span::from(format!("{:^6}", my_node.short_name))
                .white()
                .on_blue(),
        )
        .render(input_block_area_h[0], frame.buffer_mut());

        let input_width = input_block_area_h[2].width.max(1);
        let scroll = input_widget.visual_scroll(input_width as usize);

        let input = Paragraph::new(if !input_widget.value().is_empty() {
            Span::from(input_widget.value())
        } else {
            Span::from("type message...".to_owned()).dark_gray()
        })
        .scroll((0, scroll as u16));

        frame.render_widget(input, input_block_area_h[2]);

        let x = input_widget.visual_cursor().max(scroll) - scroll;
        frame.set_cursor_position((input_block_area_h[2].x + x as u16, input_block_area_h[2].y));

        let input_value_len = input_widget.value().len();

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

        Hotkeys::new(self.get_hotkeys(state)).render(state, frame, v[2]);
    }
}

struct MessageWidget<'a> {
    pub node: &'a Node,
    pub message: &'a Message,
    pub is_selected: bool,
}

impl MessageWidget<'_> {
    pub fn height(&self, width: u16) -> u16 {
        let text_height = Paragraph::new(self.message.text.clone())
            .wrap(Wrap { trim: false })
            .line_count(width - 2) as u16;

        1 + text_height + !self.message.reactions.is_empty() as u16
    }
}

impl<'a> Widget for MessageWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let text_paragraph = Paragraph::new(self.message.text.clone()).wrap(Wrap { trim: false });
        let text_height = text_paragraph.line_count(area.width - 2) as u16;

        let area = Rect {
            x: area.x,
            y: area.y,
            width: area.width - 2,
            height: 1 + text_height + !self.message.reactions.is_empty() as u16,
        };

        let block = Block::bordered()
            .borders(Borders::LEFT)
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

        let mut v_constraints = vec![Constraint::Length(1), Constraint::Length(text_height)];
        if !self.message.reactions.is_empty() {
            v_constraints.push(Constraint::Length(1));
        }

        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints(v_constraints)
            .split(block_area);

        let v0_h = Layout::default()
            .direction(Direction::Horizontal)
            .flex(layout::Flex::SpaceBetween)
            .constraints([Constraint::Fill(2), Constraint::Fill(1)])
            .split(v[0]);

        Line::from(vec![
            Span::from(format!("{:^6}", self.node.short_name))
                .black()
                .bg(if self.node.my {
                    Color::Blue
                } else {
                    Color::Green
                }),
            " ".to_span(),
            self.node.long_name.clone().to_span(),
        ])
        .render(v0_h[0], buf);

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
        .render(v0_h[1], buf);

        text_paragraph.render(v[1], buf);

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
