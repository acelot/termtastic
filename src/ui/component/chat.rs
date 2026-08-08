use crate::ui::component::Messenger;
use crate::ui::prelude::*;
use chrono::Local;

pub struct Chat<'a> {
    messenger_component: Messenger<'a>,
    list_state: ListState,
    chat_purge: Option<crate::types::Chat>,
}

impl<'a> Chat<'a> {
    pub fn new() -> Self {
        Self {
            messenger_component: Messenger::new(),
            list_state: ListState::default(),
            chat_purge: None,
        }
    }
}

impl<'a> Component for Chat<'a> {
    fn get_hotkeys(&self, state: &State) -> Vec<Hotkey> {
        if state.active_chat.is_some() {
            return self.messenger_component.get_hotkeys(state);
        }

        vec![
            Hotkey::new("↑↓", "scroll"),
            Hotkey::new("enter", "open"),
            Hotkey::new("delete", "purge chat"),
        ]
    }

    fn handle_event(
        &mut self,
        state: &State,
        event: &Event,
        emit: &impl Fn(AppEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<bool> {
        if state.active_chat.is_some() {
            return self.messenger_component.handle_event(state, event, emit);
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
                        emit(AppEvent::ChatPurgeRequested(self.chat_purge.take().unwrap()))?;
                    }
                    KeyCode::Esc => {
                        self.chat_purge = None;
                    }
                    _ => {}
                },
                _ => {}
            }

            return Ok(true);
        }

        if self.list_state.handle_navigation_events(event, Some(state.chats.len())) {
            return Ok(true);
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
                        emit(AppEvent::ChatSelected(chat.clone()))?;
                    }

                    return Ok(true);
                }
                KeyCode::Delete | KeyCode::Backspace if modifiers.is_empty() => {
                    if let Some((chat, _)) = self.list_state.selected.and_then(|i| state.chats.get_index(i)) {
                        self.chat_purge = Some(chat.clone());
                    }

                    return Ok(true);
                }
                _ => {}
            },
            _ => {}
        }

        Ok(false)
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        if state.active_chat.is_some() {
            self.messenger_component.render(state, frame, area);
            return;
        }

        if !state.chats.is_empty() {
            if let Some(selected) = self.list_state.selected
                && selected > state.chats.len() - 1
            {
                self.list_state.select(None);
            }

            if self.list_state.selected.is_none() && !state.chats.is_empty() {
                self.list_state.select(Some(0));
            }

            let list_builder = ListBuilder::new(|context| {
                let (chat, message_ids) = state.chats.get_index(context.index).unwrap();
                let last_message = message_ids.iter().last().and_then(|id| state.messages.get(id));
                let last_message_node = last_message.and_then(|message| state.nodes.get(&message.from));

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
                    is_last_message_node_my: last_message_node.map(|node| state.is_my_node(node.key))
                        .unwrap_or(false),
                    is_selected: context.is_selected,
                };

                (item, 4)
            });

            let list = ListView::new(list_builder, state.chats.len())
                .infinite_scrolling(false)
                .scrollbar(default_scrollbar());

            list.render(area, frame.buffer_mut(), &mut self.list_state);
        } else {
            PlaceholderWidget::dark_gray("no channels").render(area, frame.buffer_mut());
        }

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
    pub is_last_message_node_my: bool,
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

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
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

        // first line
        Line::from(self.chat_name_spans).render(v0_h[0], buf);
        Line::from(self.chat_type).magenta().render(v0_h[1], buf);

        Line::from(if let Some(message) = self.last_message {
            Span::from(message.datetime.with_timezone(&Local).format("%H:%M").to_string())
        } else {
            Span::from("no messages").dark_gray()
        })
        .right_aligned()
        .render(v0_h[2], buf);

        // second line
        let second_line_spans = match (self.chat, self.last_message_node, self.last_message) {
            (crate::types::Chat::Direct(_), _, Some(message)) => {
                vec![Span::from(message.text_oneline()).dark_gray()]
            }
            (_, None, Some(message)) => {
                vec![
                    short_name_to_span(&UNKNOWN_NODE, false),
                    Span::from(" "),
                    Span::from(message.text_oneline()).dark_gray(),
                ]
            }
            (_, Some(node), Some(message)) => {
                vec![
                    short_name_to_span(node, self.is_last_message_node_my),
                    Span::from(" "),
                    Span::from(message.text_oneline()).dark_gray(),
                ]
            }
            (_, _, None) => {
                vec![]
            }
        };

        Line::from(second_line_spans).render(v[1], buf);
    }
}
