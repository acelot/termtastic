use std::{collections::HashMap, iter, ops::RangeInclusive};
use tracing_unwrap::OptionExt;
use tui_widget_list::ScrollDirection;
use uuid::Uuid;

use crate::{state::ChatState, ui::prelude::*};

const INPUT_VALUE_MAX_LENGTH: usize = 200;
const VALID_INPUT_LENGTH: RangeInclusive<usize> = 1..=INPUT_VALUE_MAX_LENGTH;

pub struct Messenger<'a> {
    list_states: HashMap<Chat, ListState>,
    input_widgets: HashMap<Chat, TextArea<'a>>,
    replying_to: HashMap<Chat, (Node, Uuid)>,
    is_follow: bool,
    is_emoji_selector_visible: bool,
    emoji_selector_state: EmojiSelectorState<'a>,
    message_info: Option<Uuid>,
    message_info_state: MessageInfoWidgetState,
}

impl<'a> Messenger<'a> {
    pub fn new() -> Self {
        Self {
            list_states: HashMap::default(),
            input_widgets: HashMap::default(),
            replying_to: HashMap::default(),
            is_follow: false,
            is_emoji_selector_visible: false,
            emoji_selector_state: EmojiSelectorState::new(),
            message_info: None,
            message_info_state: MessageInfoWidgetState::default(),
        }
    }
}

impl<'a> Screen for Messenger<'a> {
    fn get_hotkeys(&self, state: &State) -> Vec<Hotkey> {
        if self.is_emoji_selector_visible {
            return [
                Some(Hotkey::new("↑↓", "scroll")),
                self.emoji_selector_state
                    .is_selected()
                    .then_some(Hotkey::new("enter", "insert")),
                Some(Hotkey::new("esc", "close")),
            ]
            .into_iter()
            .flatten()
            .collect();
        }

        if self.message_info.is_some() {
            return vec![Hotkey::new("↑↓", "scroll"), Hotkey::new("esc", "close")];
        }

        let active_chat = state.active_chat.as_ref().expect_or_log("channel should be selected");

        let input_has_single_emoji = self
            .input_widgets
            .get(active_chat)
            .and_then(|input| input.get_single_emoji())
            .is_some();

        let input_has_valid_value = self
            .input_widgets
            .get(active_chat)
            .map(|input| VALID_INPUT_LENGTH.contains(&input.trimmed_len()))
            .unwrap_or(false);

        let input_has_something = self
            .input_widgets
            .get(active_chat)
            .map(|input| !input.is_empty())
            .unwrap_or(false);

        if self.replying_to.contains_key(active_chat) {
            return [
                input_has_something.then_some(Hotkey::new("shift+enter", "new line")),
                Some(Hotkey::new("F5", "emoji")),
                input_has_single_emoji.then_some(Hotkey::new("enter", "send reaction")),
                (!input_has_single_emoji && input_has_valid_value).then_some(Hotkey::new("enter", "send reply")),
                Some(Hotkey::new("esc", "cancel reply")),
            ]
            .into_iter()
            .flatten()
            .collect();
        }

        let is_message_selected = self
            .list_states
            .get(active_chat)
            .map(|s| s.selected.is_some())
            .unwrap_or(false);

        Vec::from([
            (!input_has_something).then_some(Hotkey::new("↑↓", "scroll")),
            (is_message_selected && !input_has_something).then_some(Hotkey::new("enter", "expand")),
            (is_message_selected && !input_has_something).then_some(Hotkey::new("F2", "reply")),
            (is_message_selected && !input_has_something).then_some(Hotkey::new("F4", "info")),
            Some(Hotkey::new("F5", "emoji")),
            (is_message_selected && !input_has_something).then_some(Hotkey::new("F6", "copy")),
            input_has_something.then_some(Hotkey::new("shift+enter", "new line")),
            input_has_valid_value.then_some(Hotkey::new("enter", "send")),
            Some(Hotkey::new("esc", "channel")),
        ])
        .into_iter()
        .flatten()
        .collect()
    }

    fn handle_event(&mut self, state: &State, event: &Event, emit: &impl Fn(UiEvent)) -> bool {
        let active_chat = state.active_chat.as_ref().expect_or_log("channel should be selected");

        let input_widget = self
            .input_widgets
            .entry(*active_chat)
            .or_insert_with(|| new_input_widget());

        let is_replying_to = self.replying_to.contains_key(active_chat);

        // emoji selector
        if self.is_emoji_selector_visible {
            if self
                .emoji_selector_state
                .handle_event(event.clone(), &mut |ev| match ev {
                    EmojiSelectorWidgetEvent::EmojiSubmitted(emoji) => {
                        input_widget.insert_str(emoji.as_str());
                        self.is_emoji_selector_visible = false;
                    }
                    EmojiSelectorWidgetEvent::CloseRequested => {
                        self.is_emoji_selector_visible = false;
                    }
                })
            {
                return true;
            }
        }

        // message info
        if self.message_info.is_some() {
            if self.message_info_state.handle_event(event.clone(), &mut |ev| match ev {
                MessageInfoWidgetEvent::CloseRequested => {
                    self.message_info = None;
                }
            }) {
                return true;
            }
        }

        // replying
        if is_replying_to {
            match event {
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    modifiers,
                    ..
                }) => match code {
                    KeyCode::F(5) if modifiers.is_empty() => {
                        self.is_emoji_selector_visible = true;
                        return true;
                    }
                    KeyCode::Enter if modifiers.contains(KeyModifiers::SHIFT) => {
                        input_widget.insert_newline();
                        return true;
                    }
                    KeyCode::Enter if modifiers.is_empty() => {
                        if input_widget.trimmed_len() <= INPUT_VALUE_MAX_LENGTH
                            && let Some((_, message_id)) = self.replying_to.remove(active_chat)
                        {
                            match input_widget.get_single_emoji() {
                                Some(emoji) => {
                                    emit(UiEvent::ChatReactionSubmitted {
                                        emoji,
                                        reply_id: Some(message_id),
                                    });
                                }
                                None => {
                                    self.is_follow = true;

                                    emit(UiEvent::ChatMessageSubmitted {
                                        text: input_widget.trimmed_lines().join("\n"),
                                        reply_id: Some(message_id),
                                    });
                                }
                            }

                            input_widget.clear();

                            return true;
                        }
                    }
                    KeyCode::Esc if modifiers.is_empty() => {
                        self.replying_to.remove(active_chat);
                        return true;
                    }
                    KeyCode::Tab | KeyCode::BackTab | KeyCode::F(12) => {
                        // capture these events to prevent handling by input widget
                        return false;
                    }
                    _ => {
                        input_widget.input(event.clone());
                        return true;
                    }
                },
                Event::Paste(text) => {
                    input_widget.insert_as_lines(text);
                    return true;
                }
                _ => {
                    input_widget.input(event.clone());
                    return true;
                }
            };

            false;
        }

        let list_state = self.list_states.get_mut(active_chat).expect("should exists");
        let ChatState { ids: messages_ids, .. } = state.chats.get(active_chat).expect("chat state should exists");

        // list events
        if input_widget.is_empty() && list_state.handle_navigation_events(event, Some(messages_ids.len())) {
            if let Some(selected_index) = list_state.selected {
                if matches!(event, Event::Key(KeyEvent { code: KeyCode::Up, .. })) && selected_index == 0 {
                    emit(UiEvent::ChatTopReached);
                }

                if let Some(&selected_id) = messages_ids.get_index(selected_index) {
                    emit(UiEvent::ChatMessageSelected(*active_chat, selected_id));
                }

                self.is_follow = list_state.selected == messages_ids.len().checked_sub(1);
            }

            return true;
        }

        // default
        match event {
            Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                modifiers,
                ..
            }) => match code {
                KeyCode::Enter if modifiers.contains(KeyModifiers::SHIFT) => {
                    input_widget.insert_newline();
                    return true;
                }
                KeyCode::Enter if modifiers.is_empty() => {
                    if input_widget.is_empty() {
                        self.message_info = list_state.selected.and_then(|i| messages_ids.get_index(i).cloned());
                        return true;
                    }

                    if VALID_INPUT_LENGTH.contains(&input_widget.trimmed_len()) {
                        self.is_follow = true;

                        emit(UiEvent::ChatMessageSubmitted {
                            text: input_widget.trimmed_lines().join("\n"),
                            reply_id: None,
                        });

                        input_widget.clear();
                    }

                    return true;
                }
                KeyCode::Up if modifiers.contains(KeyModifiers::SHIFT) => {
                    if Some(0) == list_state.selected {
                        emit(UiEvent::ChatPreviousMessagesLoadRequested);
                    }

                    return true;
                }
                KeyCode::F(2) if modifiers.is_empty() => {
                    if let Some(message) = list_state
                        .selected
                        .and_then(|i| messages_ids.get_index(i))
                        .and_then(|id| state.messages.get(id))
                        && let Some(node) = state.nodes.get(&message.from)
                    {
                        self.replying_to.insert(*active_chat, (node.clone(), message.id));
                    }

                    return true;
                }
                KeyCode::F(4) if modifiers.is_empty() => {
                    if let Some(node_key) = list_state
                        .selected
                        .and_then(|i| messages_ids.get_index(i))
                        .and_then(|id| state.messages.get(id))
                        .map(|message| message.from)
                    {
                        emit(UiEvent::NodeInfoPopupOpenRequested { node_key });
                    }

                    return true;
                }
                KeyCode::F(5) if modifiers.is_empty() => {
                    self.is_emoji_selector_visible = true;
                    return true;
                }
                KeyCode::F(6) if modifiers.is_empty() => {
                    if let Some(message) = list_state
                        .selected
                        .and_then(|i| messages_ids.get_index(i))
                        .and_then(|id| state.messages.get(id))
                    {
                        emit(UiEvent::CopyToClipboardRequested(message.text.clone()));
                    }

                    return true;
                }
                KeyCode::Esc if modifiers.is_empty() => {
                    emit(UiEvent::ChatSwitchRequested);
                    self.is_follow = false;
                    return true;
                }
                KeyCode::Tab | KeyCode::BackTab => {
                    self.is_follow = false;
                    return false;
                }
                KeyCode::F(12) => {
                    // capture these events to prevent handling by input widget
                    return false;
                }
                _ => {}
            },
            Event::Paste(text) => {
                input_widget.insert_as_lines(text);
                return true;
            }
            Event::FocusLost => {
                self.is_follow = false;
                return true;
            }
            Event::FocusGained => {
                if list_state.selected == messages_ids.len().checked_sub(1) {
                    self.is_follow = true;
                    return true;
                }
            }
            _ => {}
        }

        input_widget.input(event.clone());

        true
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect, emit: &impl Fn(UiEvent)) {
        let active_chat = state.active_chat.as_ref().expect_or_log("channel should be selected");
        let chat_state = state.chats.get(active_chat).expect_or_log("chat state should exists");
        let replying_to = self.replying_to.get(active_chat);
        let is_any_popup_visible =
            state.nodeinfo_key.is_some() || self.is_emoji_selector_visible || self.message_info.is_some();

        let list_state = self.list_states.entry(*active_chat).or_insert_with(|| {
            let mut list_state = ListState::default();

            if !chat_state.position.selected_id.is_nil() {
                list_state.select(chat_state.ids.get_index_of(&chat_state.position.selected_id));
            }

            list_state
        });

        let input_widget = self
            .input_widgets
            .entry(*active_chat)
            .or_insert_with(|| new_input_widget());

        let v = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(2 + input_widget.lines().len().min(5) as u16),
        ])
        .split(area);

        // list
        if list_state.fix_selection(chat_state.ids.len()) {
            if list_state.selected == chat_state.ids.len().checked_sub(1) {
                self.is_follow = true;
            }
        }

        if self.is_follow {
            let index = chat_state.ids.len().checked_sub(1);

            if list_state.selected != index {
                list_state.select(index);

                if let Some(selected_id) = index.and_then(|i| chat_state.ids.get_index(i)) {
                    emit(UiEvent::ChatMessageSelected(*active_chat, *selected_id));
                }
            }
        }

        if !chat_state.ids.is_empty() {
            let list_builder = ListBuilder::new(|context| {
                let message_id = chat_state
                    .ids
                    .get_index(context.index)
                    .expect_or_log("message should exist");

                let item = MessageWidget::new(
                    MessageWidgetData::new(*message_id, state)
                        .selected(context.is_selected)
                        .highlighted(replying_to.map(|(_, msg_key)| message_id == msg_key).unwrap_or(false)),
                );

                let (height, _) = item.height(area.width);

                (item, height)
            });

            let list = ListView::new(list_builder, chat_state.ids.len())
                .gap_size(1)
                .infinite_scrolling(false)
                .scroll_direction(ScrollDirection::Backward)
                .scrollbar(default_scrollbar())
                .add_modifier(if is_any_popup_visible {
                    Modifier::DIM
                } else {
                    Modifier::empty()
                });

            list.render(v[0], frame.buffer_mut(), list_state);
        } else {
            PlaceholderWidget::dark_gray("no messages").render(v[0], frame.buffer_mut());
        }

        // input
        let input_block = Block::bordered()
            .padding(Padding::symmetric(1, 0))
            .border_type(BorderType::Rounded)
            .border_style(Style::new().dark_gray())
            .add_modifier(if is_any_popup_visible {
                Modifier::DIM
            } else {
                Modifier::empty()
            });

        let input_block_area = input_block.inner(v[1]);

        let channel_name_spans = match (active_chat, replying_to) {
            (Chat::Channel(_), None) => chat_to_spans(active_chat, state)
                .iter()
                .chain(iter::once(&Span::from(" ←").dark_gray()))
                .cloned()
                .collect(),
            (Chat::Direct(node_key), None) => {
                let node = state.nodes.get(node_key).unwrap_or(&UNKNOWN_NODE);

                vec![
                    short_name_to_span(node, state.is_my_node(node.key)),
                    Span::from(" ←").dark_gray(),
                ]
            }
            (_, Some((node, _))) => vec![
                Span::from("reply to ").cyan(),
                short_name_to_span(node, state.is_my_node(node.key)),
                Span::from(" ←").dark_gray(),
            ],
        };

        let channel_line = Line::from(channel_name_spans);

        let input_block_area_h = Layout::horizontal([
            Constraint::Length(channel_line.width() as u16),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(8),
        ])
        .split(input_block_area);

        input_block.render(v[1], frame.buffer_mut());
        channel_line.render(input_block_area_h[0], frame.buffer_mut());
        frame.render_widget(&*input_widget, input_block_area_h[2]);

        let input_value_len = input_widget.trimmed_len();

        Line::from(
            Span::from(format!("{}/{}", input_value_len, INPUT_VALUE_MAX_LENGTH)).style(Style::new().fg(
                if input_value_len > INPUT_VALUE_MAX_LENGTH {
                    Color::Red
                } else {
                    Color::DarkGray
                },
            )),
        )
        .right_aligned()
        .render(input_block_area_h[3], frame.buffer_mut());

        // unread indicator
        let unread_messages_count = state.unread_messages_count_by_chat(*active_chat);

        if unread_messages_count > 0 {
            let unread_span = Span::from(format!(" ↓ unread: {} ", unread_messages_count))
                .white()
                .on_blue();

            let unread_popup_area = Rect::new(area.x, area.y + area.height - 5, area.width, 1)
                .centered_horizontally(Constraint::Length(unread_span.width() as u16));

            unread_span.render(unread_popup_area, frame.buffer_mut());
        }

        // emoji selector
        if self.is_emoji_selector_visible {
            let popup_area = v[0].centered(Constraint::Length(40), Constraint::Length(14));

            Clear.render(popup_area.outer(Margin::new(1, 0)), frame.buffer_mut());

            EmojiSelectorWidget::new().render(popup_area, frame.buffer_mut(), &mut self.emoji_selector_state);
        }

        // message info
        if let Some(message_id) = self.message_info {
            let popup_area = area.centered(Constraint::Max(80), Constraint::Max(21));

            Clear.render(popup_area.outer(Margin::new(1, 0)), frame.buffer_mut());

            MessageInfoWidget::new(MessageInfoWidgetData::new(message_id, state)).render(
                popup_area,
                frame.buffer_mut(),
                &mut self.message_info_state,
            );
        }
    }
}

fn new_input_widget() -> TextArea<'static> {
    let mut input = TextArea::default();
    input.set_placeholder_text("type message...");
    input.set_cursor_line_style(Style::default());

    input
}
