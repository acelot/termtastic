use ordermap::OrderSet;
use std::{collections::HashMap, iter, ops::RangeInclusive, sync::LazyLock};
use tracing_unwrap::OptionExt;
use tui_widget_list::ScrollDirection;
use uuid::Uuid;

use crate::ui::prelude::*;

const INPUT_VALUE_MAX_LENGTH: usize = 200;
const VALID_INPUT_LENGTH: RangeInclusive<usize> = 1..=INPUT_VALUE_MAX_LENGTH;

static EMPTY_MESSAGES: LazyLock<OrderSet<Uuid>> = LazyLock::new(|| OrderSet::new());

pub struct Messenger<'a> {
    list_states: HashMap<Chat, ListState>,
    input_widgets: HashMap<Chat, TextArea<'a>>,
    follow_chat: HashMap<Chat, bool>,
    replying_to: HashMap<Chat, (Node, Uuid)>,
    is_emoji_selector_visible: bool,
    emoji_selector_state: EmojiSelectorState<'a>,
    is_reaction_viewer_visible: bool,
    reactions_viewer_state: ReactionViewerState,
}

impl<'a> Messenger<'a> {
    pub fn new() -> Self {
        Self {
            list_states: HashMap::default(),
            input_widgets: HashMap::default(),
            follow_chat: HashMap::default(),
            replying_to: HashMap::default(),
            emoji_selector_state: EmojiSelectorState::new(),
            is_emoji_selector_visible: false,
            reactions_viewer_state: ReactionViewerState::new(),
            is_reaction_viewer_visible: false,
        }
    }
}

impl<'a> Screen for Messenger<'a> {
    fn get_hotkeys(&self, state: &State) -> Vec<Hotkey> {
        if self.is_reaction_viewer_visible {
            return vec![Hotkey::new("↑↓", "scroll"), Hotkey::new("esc", "close")];
        }

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
            (is_message_selected && !input_has_something).then_some(Hotkey::new("F2", "reply")),
            (is_message_selected && !input_has_something).then_some(Hotkey::new("F4", "info")),
            Some(Hotkey::new("F5", "emoji")),
            (is_message_selected && !input_has_something).then_some(Hotkey::new("F6", "copy")),
            (is_message_selected && !input_has_something).then_some(Hotkey::new("F7", "reactions")),
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

        let list_state = self.list_states.entry(active_chat.clone()).or_default();

        let input_widget = self
            .input_widgets
            .entry(active_chat.clone())
            .or_insert_with(|| new_input_widget());

        let is_replying_to = self.replying_to.contains_key(active_chat);

        let messages: Vec<&Message> = state
            .chats
            .get(active_chat)
            .map(|ids| ids.iter().filter_map(|id| state.messages.get(id)).collect())
            .unwrap_or_default();

        if self.is_reaction_viewer_visible {
            match event {
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    modifiers,
                    ..
                }) if modifiers.is_empty() => match code {
                    KeyCode::Esc => {
                        self.is_reaction_viewer_visible = false;
                        return true;
                    }
                    _ => {}
                },
                _ => {}
            };

            let reactions_count = list_state
                .selected
                .and_then(|i| messages.get(i))
                .map(|m| {
                    m.reactions.iter().fold(0, |mut counter, message_id| {
                        if state.messages.contains_key(message_id) {
                            counter += 1;
                        };

                        counter
                    })
                })
                .unwrap_or(0);

            return self.reactions_viewer_state.handle_event(event, reactions_count);
        }

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
                                    emit(UiEvent::ChatMessageSubmitted {
                                        text: input_widget.trimmed_lines().join("\n"),
                                        reply_id: Some(message_id),
                                    });

                                    self.follow_chat.insert(active_chat.clone(), true);
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
                    KeyCode::Tab | KeyCode::BackTab | KeyCode::Esc | KeyCode::F(12) => {
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
                }
                _ => {
                    input_widget.input(event.clone());
                    return true;
                }
            };

            false;
        }

        if input_widget.is_empty() && list_state.handle_navigation_events(event, Some(messages.len())) {
            if let Some(index) = list_state.selected {
                if matches!(event, Event::Key(KeyEvent { code: KeyCode::Up, .. })) && index == 0 {
                    emit(UiEvent::ChatTopReached);
                }

                self.follow_chat
                    .insert(active_chat.clone(), Some(index) == messages.len().checked_sub(1));
            }

            return true;
        }

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
                    if VALID_INPUT_LENGTH.contains(&input_widget.trimmed_len()) {
                        self.follow_chat.insert(active_chat.clone(), true);

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
                    if let Some(message) = list_state.selected.and_then(|i| messages.get(i))
                        && let Some(node) = state.nodes.get(&message.from)
                    {
                        self.replying_to.insert(active_chat.clone(), (node.clone(), message.id));
                    }

                    return true;
                }
                KeyCode::F(4) if modifiers.is_empty() => {
                    if let Some(node_key) = list_state
                        .selected
                        .and_then(|i| messages.get(i))
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
                    if let Some(message) = list_state.selected.and_then(|i| messages.get(i)) {
                        emit(UiEvent::CopyToClipboardRequested(message.text.clone()));
                    }

                    return true;
                }
                KeyCode::F(7) if modifiers.is_empty() => {
                    if list_state.selected.and_then(|i| messages.get(i)).is_some() {
                        self.follow_chat.insert(active_chat.clone(), false);
                        self.is_reaction_viewer_visible = true;
                    }

                    return true;
                }
                KeyCode::Esc if modifiers.is_empty() => {
                    self.follow_chat.insert(active_chat.clone(), false);
                    emit(UiEvent::ChatSwitchRequested);
                    return true;
                }
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Esc | KeyCode::F(12) => {
                    // capture these events to prevent handling by input widget
                    return false;
                }
                _ => {}
            },
            Event::Paste(text) => {
                input_widget.insert_as_lines(text);
            }
            _ => {}
        }

        input_widget.input(event.clone());

        true
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        let active_chat = state.active_chat.as_ref().expect_or_log("channel should be selected");

        if !self.list_states.contains_key(active_chat) {
            self.list_states.insert(active_chat.clone(), ListState::default());
        }

        if !self.input_widgets.contains_key(active_chat) {
            self.input_widgets.insert(active_chat.clone(), new_input_widget());
        }

        if !self.follow_chat.contains_key(active_chat) {
            self.follow_chat.insert(active_chat.clone(), true);
        }

        let list_state = self.list_states.get_mut(active_chat).unwrap();
        let input_widget = self.input_widgets.get_mut(active_chat).unwrap();
        let follow_chat = self.follow_chat.get(active_chat).unwrap();
        let replying_to = self.replying_to.get(active_chat);
        let message_ids = state.chats.get(active_chat).unwrap_or(&EMPTY_MESSAGES);

        list_state.fix_selection(message_ids.len());

        if *follow_chat && !message_ids.is_empty() {
            list_state.select(Some(message_ids.len() - 1));
        }

        let v = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(2 + input_widget.lines().len().min(5) as u16),
        ])
        .split(area);

        let is_any_popup_visible =
            state.nodeinfo_key.is_some() || self.is_emoji_selector_visible || self.is_reaction_viewer_visible;

        // list
        if !message_ids.is_empty() {
            let list_builder = ListBuilder::new(|context| {
                let message = message_ids
                    .get_index(context.index)
                    .and_then(|id| state.messages.get(id))
                    .expect_or_log("message should exist");

                let reactions: Vec<&Message> = message
                    .reactions
                    .iter()
                    .filter_map(|message_id| state.messages.get(message_id))
                    .collect();

                let replying_node = if !message.reply_id.is_nil() {
                    state
                        .messages
                        .get(&message.reply_id)
                        .map(|m| state.nodes.get(&m.from).unwrap_or(&UNKNOWN_NODE))
                } else {
                    None
                };

                let replying_message = if !message.reply_id.is_nil() {
                    state.messages.get(&message.reply_id)
                } else {
                    None
                };

                let node = state.nodes.get(&message.from).unwrap_or(&UNKNOWN_NODE);

                let item = MessageWidget::new(
                    node,
                    message,
                    reactions,
                    replying_node,
                    replying_message,
                    state.is_my_node(node.key),
                    context.is_selected,
                    replying_to.map(|(_, msg_key)| message.id == *msg_key).unwrap_or(false),
                );

                let (mut height, _) = item.height(area.width);

                if context.index < message_ids.len() - 1 {
                    height += 1;
                }

                (item, height)
            });

            let list = ListView::new(list_builder, message_ids.len())
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

        // reaction viewer
        if self.is_reaction_viewer_visible
            && let Some(message) = list_state
                .selected
                .and_then(|i| message_ids.get_index(i))
                .and_then(|id| state.messages.get(id))
        {
            let popup_area = v[0].centered(Constraint::Length(50), Constraint::Length(15));

            Clear.render(popup_area.outer(Margin::new(1, 0)), frame.buffer_mut());

            self.is_reaction_viewer_visible = true;

            let reaction_items: Vec<ReactionViewerItem> = message
                .reactions
                .iter()
                .filter_map(|message_id| {
                    let Some(reaction) = state.messages.get(message_id) else {
                        return None;
                    };

                    let node = state.nodes.get(&reaction.from).unwrap_or(&UNKNOWN_NODE);

                    Some(ReactionViewerItem {
                        reaction,
                        node,
                        is_my_node: state.is_my_node(node.key),
                    })
                })
                .collect();

            ReactionViewerWidget::new(reaction_items).render(
                popup_area,
                frame.buffer_mut(),
                &mut self.reactions_viewer_state,
            );
        }

        // emoji selector
        if self.is_emoji_selector_visible {
            let popup_area = v[0].centered(Constraint::Length(40), Constraint::Length(14));

            Clear.render(popup_area.outer(Margin::new(1, 0)), frame.buffer_mut());

            EmojiSelectorWidget::new().render(popup_area, frame.buffer_mut(), &mut self.emoji_selector_state);
        }
    }
}

fn new_input_widget() -> TextArea<'static> {
    let mut input = TextArea::default();
    input.set_placeholder_text("type message...");
    input.set_cursor_line_style(Style::default());

    input
}
