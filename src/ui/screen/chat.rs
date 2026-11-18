use crate::state::ChatState;
use crate::ui::prelude::*;
use crate::ui::screen::Messenger;

pub struct Chat<'a> {
    messenger_screen: Messenger<'a>,
    list_state: ListState,
    chat_purge: Option<crate::types::Chat>,
}

impl<'a> Chat<'a> {
    pub fn new() -> Self {
        Self {
            messenger_screen: Messenger::new(),
            list_state: ListState::default(),
            chat_purge: None,
        }
    }
}

impl<'a> Screen for Chat<'a> {
    fn get_hotkeys(&self, state: &State) -> Vec<Hotkey> {
        if state.active_chat.is_some() {
            return self.messenger_screen.get_hotkeys(state);
        }

        [
            (!state.chats.is_empty()).then_some(Hotkey::new("↑↓", "scroll")),
            (!state.chats.is_empty()).then_some(Hotkey::new("enter", "open")),
            self.list_state
                .selected
                .is_some()
                .then_some(Hotkey::new("delete", "purge chat")),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn handle_event(&mut self, state: &State, event: &Event, emit: &impl Fn(UiEvent)) -> bool {
        if state.active_chat.is_some() {
            return self.messenger_screen.handle_event(state, event, emit);
        }

        if self.chat_purge.is_some() {
            match event {
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    modifiers,
                    ..
                }) if modifiers.is_empty() => match code {
                    KeyCode::Enter => {
                        emit(UiEvent::ChatPurgeRequested(self.chat_purge.take().unwrap()));
                    }
                    KeyCode::Esc => {
                        self.chat_purge = None;
                    }
                    _ => {}
                },
                _ => {}
            }

            return true;
        }

        if self.list_state.handle_navigation_events(event, Some(state.chats.len())) {
            return true;
        }

        match event {
            Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                modifiers,
                ..
            }) => match code {
                KeyCode::Enter if modifiers.is_empty() => {
                    if let Some((chat, _)) = self.list_state.selected.and_then(|i| state.chats.get_index(i)) {
                        emit(UiEvent::ChatSelected(*chat));
                    }

                    return true;
                }
                KeyCode::Delete | KeyCode::Backspace if modifiers.is_empty() => {
                    if let Some((chat, _)) = self.list_state.selected.and_then(|i| state.chats.get_index(i)) {
                        self.chat_purge = Some(*chat);
                    }

                    return true;
                }
                _ => {}
            },
            _ => {}
        }

        false
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect, emit: &impl Fn(UiEvent)) {
        if state.active_chat.is_some() {
            self.messenger_screen.render(state, frame, area, emit);
            return;
        }

        self.list_state.fix_selection(state.chats.len());

        if state.chats.is_empty() {
            PlaceholderWidget::dark_gray("no channels").render(area, frame.buffer_mut());
            return;
        }

        let list_builder = ListBuilder::new(|context| {
            let (chat, ChatState { ids: message_ids, .. }) = state.chats.get_index(context.index).unwrap();
            let last_message = message_ids.iter().last().and_then(|id| state.messages.get(id));
            let last_message_node = last_message.and_then(|message| state.nodes.get(&message.from));
            let unread_messages_count = state.unread_messages_count_by_chat(*chat);

            let chat_type = match chat {
                crate::types::Chat::Channel(channel_key) => {
                    let channel = state.channels.get(channel_key).expect("should be Some");

                    match channel.role {
                        ChannelRole::Primary => "PRIMARY",
                        ChannelRole::Secondary => "SECONDARY",
                        ChannelRole::Disabled => "DISABLED",
                    }
                }
                crate::types::Chat::Direct(_) => "DIRECT",
            };

            let item = ChannelWidget {
                chat,
                chat_type,
                chat_name_spans: chat_to_spans(chat, state),
                last_message,
                last_message_node,
                unread_messages_count,
                is_last_message_node_my: last_message_node
                    .map(|node| state.is_my_node(node.key))
                    .unwrap_or(false),
                is_selected: context.is_selected,
            };

            (item, 4)
        });

        let list = ListView::new(list_builder, state.chats.len())
            .infinite_scrolling(false)
            .scrollbar(default_scrollbar());

        list.render(area, frame.buffer_mut(), &mut self.list_state);

        if let Some(chat_purge) = &self.chat_purge {
            PopupConfirmWidget::new(
                if chat_purge.is_direct() {
                    "Are you sure to delete the chat?"
                } else {
                    "Are you sure to clear the channel messages?"
                },
                "confirm",
                "cancel",
                40,
                Color::Red,
            )
            .render(area, frame.buffer_mut());
        }
    }
}

struct ChannelWidget<'a> {
    pub chat: &'a crate::types::Chat,
    pub chat_type: &'a str,
    pub chat_name_spans: Vec<Span<'a>>,
    pub last_message: Option<&'a Message>,
    pub last_message_node: Option<&'a Node>,
    pub unread_messages_count: usize,
    pub is_last_message_node_my: bool,
    pub is_selected: bool,
}

impl<'a> Widget for ChannelWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let unread_message_count_length = self.unread_messages_count.checked_ilog10().unwrap_or(0) as u16 + 1;

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

        let v = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(block_area);
        let v0_h = Layout::horizontal([Constraint::Fill(3), Constraint::Fill(1), Constraint::Fill(1)]).split(v[0]);
        let v1_h = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(if unread_message_count_length > 0 {
                unread_message_count_length + 4
            } else {
                0
            }),
        ])
        .split(v[1]);

        // first line
        Line::from(self.chat_name_spans).render(v0_h[0], buf);
        Line::from(self.chat_type).magenta().render(v0_h[1], buf);

        Line::from(if let Some(message) = self.last_message {
            Span::from(humanize_datetime(to_local(message.datetime)))
        } else {
            Span::from("no messages").dark_gray()
        })
        .right_aligned()
        .render(v0_h[2], buf);

        // second line
        let message_color = if self.unread_messages_count > 0 {
            Color::White
        } else {
            Color::DarkGray
        };

        let second_line_spans = match (self.chat, self.last_message_node, self.last_message) {
            (crate::types::Chat::Direct(_), _, Some(message)) => {
                vec![Span::from(message.text_oneline()).fg(message_color)]
            }
            (_, None, Some(message)) => {
                vec![
                    short_name_to_span(&UNKNOWN_NODE, false),
                    Span::from(" "),
                    Span::from(message.text_oneline()).fg(message_color),
                ]
            }
            (_, Some(node), Some(message)) => {
                vec![
                    short_name_to_span(node, self.is_last_message_node_my),
                    Span::from(" "),
                    Span::from(message.text_oneline()).fg(message_color),
                ]
            }
            (_, _, None) => {
                vec![]
            }
        };

        Line::from(second_line_spans).render(v1_h[0], buf);

        if self.unread_messages_count > 0 {
            Line::from(
                Span::from(format!(" {} ", self.unread_messages_count))
                    .white()
                    .on_blue(),
            )
            .right_aligned()
            .render(v1_h[1], buf);
        }
    }
}
