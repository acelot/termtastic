use crossterm::event::KeyModifiers;
use itertools::Itertools;
use ordermap::OrderMap;
use std::{collections::HashMap, iter, ops::RangeInclusive};
use tracing_unwrap::OptionExt;
use tui_widget_list::ScrollDirection;

use crate::ui::prelude::*;

const INPUT_VALUE_MAX_LENGTH: usize = 200;
const VALID_INPUT_LENGTH: RangeInclusive<usize> = 1..=INPUT_VALUE_MAX_LENGTH;
const REACTIONS_LINE_MAX_WIDTH: usize = 20;

pub struct Messenger<'a> {
    list_states: HashMap<Chat, ListState>,
    input_widgets: HashMap<Chat, TextArea<'a>>,
    follow_chat: HashMap<Chat, bool>,
    replying_to: HashMap<Chat, (Node, u32)>,
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

impl<'a> Component for Messenger<'a> {
    fn get_hotkeys(&self, state: &State) -> Vec<Hotkey> {
        if self.is_reaction_viewer_visible {
            return vec![Hotkey::new("↑↓", "scroll"), Hotkey::new("esc", "close")];
        }

        if self.is_emoji_selector_visible {
            return vec![
                Hotkey::new("↑↓", "scroll"),
                Hotkey::new("enter", "insert"),
                Hotkey::new("esc", "close"),
            ];
        }

        let active_chat = state.active_chat.as_ref().expect_or_log("channel should be selected");

        let input_has_single_emoji = self
            .input_widgets
            .get(active_chat)
            .and_then(|input| input.get_single_emoji())
            .is_some();

        let input_has_valid_value = self
            .input_widgets
            .get(active_chat).map(|input| VALID_INPUT_LENGTH.contains(&input.trimmed_len()))
            .unwrap_or(false);

        let input_has_something = self
            .input_widgets
            .get(active_chat).map(|input| !input.is_empty())
            .unwrap_or(false);

        if self.replying_to.contains_key(active_chat) {
            return vec![
                input_has_something.then_some(Hotkey::new(
                    if cfg!(target_os = "macos") {
                        "⌥ enter"
                    } else {
                        "alt+enter"
                    },
                    "new line",
                )),
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
            .get(active_chat).map(|s| s.selected.is_some())
            .unwrap_or(false);

        Vec::from([
            (!input_has_something).then_some(Hotkey::new("↑↓", "scroll")),
            (is_message_selected && !input_has_something).then_some(Hotkey::new("F2", "reply")),
            (is_message_selected && !input_has_something).then_some(Hotkey::new("F4", "node info")),
            Some(Hotkey::new("F5", "emoji")),
            (is_message_selected && !input_has_something).then_some(Hotkey::new("F7", "reactions")),
            input_has_something.then_some(Hotkey::new("shift+enter", "new line")),
            input_has_valid_value.then_some(Hotkey::new("enter", "send")),
            Some(Hotkey::new("esc", "switch channel")),
        ])
        .into_iter()
        .flatten()
        .collect()
    }

    fn handle_event(
        &mut self,
        state: &State,
        event: &Event,
        emit: &impl Fn(AppEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<bool> {
        let active_chat = state.active_chat.as_ref().expect_or_log("channel should be selected");

        let list_state = self
            .list_states
            .entry(active_chat.clone())
            .or_default();

        let input_widget = self
            .input_widgets
            .entry(active_chat.clone())
            .or_insert_with(|| new_input_widget());

        let is_replying_to = self.replying_to.contains_key(active_chat);

        let messages: Vec<&Message> = state
            .chats
            .get(active_chat).map(|ids| ids.iter().filter_map(|id| state.messages.get(id)).collect())
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
                        return Ok(true);
                    }
                    _ => {}
                },
                _ => {}
            };

            let reactions_count = list_state
                .selected
                .and_then(|i| messages.get(i)).map(|m| m.reactions.iter().fold(0, |mut counter, message_id| {
                        if state.messages.contains_key(message_id) {
                            counter += 1;
                        };

                        counter
                    }))
                .unwrap_or(0);

            return self.reactions_viewer_state.handle_event(event, reactions_count);
        }

        if self.is_emoji_selector_visible {
            match event {
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    modifiers,
                    ..
                }) if modifiers.is_empty() => match code {
                    KeyCode::Enter => {
                        if let Some(emoji) = self.emoji_selector_state.get_value() {
                            input_widget.insert_str(emoji.glyph);
                            self.is_emoji_selector_visible = false;
                            return Ok(true);
                        }
                    }
                    KeyCode::Esc => {
                        self.is_emoji_selector_visible = false;
                        self.emoji_selector_state.reset();
                        return Ok(true);
                    }
                    _ => {}
                },
                _ => {}
            };

            return self.emoji_selector_state.handle_event(event.clone());
        }

        if is_replying_to {
            match event {
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    modifiers,
                    ..
                }) if modifiers.is_empty() => match code {
                    KeyCode::F(5) => {
                        self.is_emoji_selector_visible = true;
                        return Ok(true);
                    }
                    KeyCode::Enter if modifiers.contains(KeyModifiers::SHIFT) => {
                        input_widget.insert_newline();
                        return Ok(true);
                    }
                    KeyCode::Enter => {
                        if input_widget.trimmed_len() <= INPUT_VALUE_MAX_LENGTH
                            && let Some((_, message_id)) = self.replying_to.remove(active_chat)
                        {
                            self.follow_chat.insert(active_chat.clone(), true);

                            match input_widget.get_single_emoji() {
                                Some(emoji) => {
                                    emit(AppEvent::ChatReactionSubmitted {
                                        emoji,
                                        reply_message_id: Some(message_id),
                                    })?;
                                }
                                None => {
                                    emit(AppEvent::ChatMessageSubmitted {
                                        text: input_widget.trimmed_lines().join("\n"),
                                        reply_message_id: Some(message_id),
                                    })?;
                                }
                            }

                            input_widget.clear();

                            return Ok(true);
                        }
                    }
                    KeyCode::Esc => {
                        self.replying_to.remove(active_chat);
                        return Ok(true);
                    }
                    _ => {}
                },
                _ => {}
            };

            input_widget.input(event.clone());

            return Ok(true);
        }

        if input_widget.is_empty() && list_state.handle_navigation_events(event, Some(messages.len())) {
            if let Some(index) = list_state.selected {
                self.follow_chat
                    .insert(active_chat.clone(), index == messages.len() - 1);
            }

            return Ok(true);
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
                    return Ok(true);
                }
                KeyCode::Enter if modifiers.is_empty() => {
                    if input_widget.trimmed_len() <= INPUT_VALUE_MAX_LENGTH {
                        self.follow_chat.insert(active_chat.clone(), true);

                        emit(AppEvent::ChatMessageSubmitted {
                            text: input_widget.trimmed_lines().join("\n"),
                            reply_message_id: None,
                        })?;

                        input_widget.clear();
                    }

                    return Ok(true);
                }
                KeyCode::F(2) if modifiers.is_empty() => {
                    if let Some(message) = list_state.selected.and_then(|i| messages.get(i))
                        && let Some(node) = state.nodes.get(&message.from) {
                            self.replying_to.insert(active_chat.clone(), (node.clone(), message.id));
                        }

                    return Ok(true);
                }
                KeyCode::F(4) if modifiers.is_empty() => {
                    if let Some(node_key) = list_state
                        .selected
                        .and_then(|i| messages.get(i)).map(|message| message.from)
                    {
                        emit(AppEvent::NodeInfoPopupOpenRequested(node_key))?;
                    }

                    return Ok(true);
                }
                KeyCode::F(5) if modifiers.is_empty() => {
                    self.is_emoji_selector_visible = true;
                    return Ok(true);
                }
                KeyCode::F(7) if modifiers.is_empty() => {
                    if list_state.selected.and_then(|i| messages.get(i)).is_some() {
                        self.follow_chat.insert(active_chat.clone(), false);
                        self.is_reaction_viewer_visible = true;
                    }

                    return Ok(true);
                }
                KeyCode::Esc if modifiers.is_empty() => {
                    emit(AppEvent::ChatSwitchRequested)?;
                    return Ok(true);
                }
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Esc if modifiers.is_empty() => {
                    // capture these events to prevent handling by input widget
                    return Ok(false);
                }
                _ => {}
            },
            Event::Paste(text) => {
                input_widget.insert_as_lines(text);
            }
            _ => {}
        }

        input_widget.input(event.clone());

        Ok(true)
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
        let message_ids = state.chats.get(active_chat).map(|ids| ids.as_slice()).unwrap_or(&[]);

        if *follow_chat && !message_ids.is_empty() {
            list_state.select(Some(message_ids.len() - 1));
        }

        let v = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(2 + input_widget.lines().len().min(5) as u16),
        ])
        .split(area);

        let is_any_popup_visible =
            state.nodeinfo.is_some() || self.is_emoji_selector_visible || self.is_reaction_viewer_visible;

        // list
        if !message_ids.is_empty() {
            let list_builder = ListBuilder::new(|context| {
                let message = message_ids
                    .get(context.index)
                    .and_then(|id| state.messages.get(id))
                    .expect_or_log("message should exist");

                let reactions: Vec<&Message> = message
                    .reactions
                    .iter()
                    .filter_map(|message_id| state.messages.get(message_id))
                    .collect();

                let replying_node = if message.reply_message_id > 0 {
                    state
                        .messages
                        .get(&message.reply_message_id).map(|m| state.nodes.get(&m.from).unwrap_or(&UNKNOWN_NODE))
                } else {
                    None
                };

                let replying_message = if message.reply_message_id > 0 {
                    state.messages.get(&message.reply_message_id)
                } else {
                    None
                };

                let node = state.nodes.get(&message.from).unwrap_or(&UNKNOWN_NODE);

                let item = MessageWidget {
                    node,
                    message,
                    message_paragraph: MessageWidget::get_text_paragraph(message, replying_message),
                    reactions_line: MessageWidget::get_reactions_line(reactions),
                    replying_node,
                    is_my_node: state.is_my_node(node.key),
                    is_selected: context.is_selected,
                    is_highlighted: replying_to.map(|(_, msg_key)| message.id == *msg_key)
                        .unwrap_or(false),
                };

                let mut height = item.text_height(area.width) + 1;

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
                .and_then(|i| message_ids.get(i))
                .and_then(|id| state.messages.get(id))
        {
            let popup_area = v[0].centered(Constraint::Length(40), Constraint::Length(14));

            Clear.render(popup_area, frame.buffer_mut());

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

            Clear.render(popup_area, frame.buffer_mut());

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

struct MessageWidget<'a> {
    pub node: &'a Node,
    pub message: &'a Message,
    pub message_paragraph: Paragraph<'a>,
    pub reactions_line: Line<'a>,
    pub replying_node: Option<&'a Node>,
    pub is_my_node: bool,
    pub is_selected: bool,
    pub is_highlighted: bool,
}

impl MessageWidget<'_> {
    pub fn text_height(&self, width: u16) -> u16 {
        self.message_paragraph.line_count(
            width
                .saturating_sub(self.reactions_line.width().min(REACTIONS_LINE_MAX_WIDTH) as u16)
                .saturating_sub(4),
        ) as u16
    }

    pub fn get_text_paragraph<'a>(message: &'a Message, replied_message: Option<&'a Message>) -> Paragraph<'a> {
        let reply_line = replied_message.map(|msg| Line::from(vec!["“".to_span(), Span::from(msg.text_oneline()), "”".to_span()]).magenta());

        let text_lines: Vec<Line<'_>> = message.text.to_hyperlinked_lines();

        Paragraph::new(reply_line.into_iter().chain(text_lines).collect::<Vec<Line<'_>>>()).wrap(Wrap { trim: false })
    }

    pub fn get_reactions_line(reactions: Vec<&'_ Message>) -> Line<'_> {
        let summary = reactions
            .iter()
            .sorted_by_key(|r| r.datetime)
            .fold(OrderMap::new(), |mut acc, r| {
                *acc.entry(&r.text).or_insert(0) += 1;
                acc
            });

        Line::from(
            summary
                .into_iter()
                .flat_map(|(emoji, nodes_count)| {
                    if nodes_count > 1 {
                        vec![
                            " ".to_span(),
                            emoji.to_span(),
                            Span::from(format!(":{}", nodes_count)).dark_gray(),
                        ]
                    } else {
                        vec![" ".to_span(), emoji.to_span()]
                    }
                })
                .collect::<Vec<Span>>(),
        )
        .right_aligned()
    }
}

impl<'a> Widget for MessageWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let text_height = self.text_height(area.width);

        let area = Rect {
            x: area.x,
            y: area.y,
            width: area.width - 2,
            height: text_height + 1,
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

        let v = Layout::vertical([Constraint::Length(1), Constraint::Length(text_height)]).split(block_area);

        let v0_h = Layout::horizontal([Constraint::Fill(4), Constraint::Fill(2), Constraint::Fill(1)])
            .flex(Flex::SpaceBetween)
            .split(v[0]);

        let v1_h = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(2),
            Constraint::Length(self.reactions_line.width().min(REACTIONS_LINE_MAX_WIDTH) as u16),
        ])
        .split(v[1]);

        // first line
        if let Some(rep_node) = self.replying_node {
            Line::from(vec![
                short_name_to_span(self.node, self.is_my_node),
                " → ".to_span().dark_gray(),
                short_name_to_span(rep_node, self.is_my_node).on_magenta(),
            ])
            .render(v0_h[0], buf);
        } else {
            Line::from(vec![
                short_name_to_span(self.node, self.is_my_node),
                " ".to_span(),
                self.node.long_name().to_span().bold(),
            ])
            .render(v0_h[0], buf);
        }

        if !self.is_my_node {
            Line::from(hops_to_spans(self.message, false)).render(v0_h[1], buf);
        } else {
            routing_error_to_span(self.message.routing_error).render(v0_h[1], buf);
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
        self.message_paragraph.render(v1_h[0], buf);
        self.reactions_line.render(v1_h[2], buf);
    }
}
