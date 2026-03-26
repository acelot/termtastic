use std::collections::VecDeque;

use tracing_unwrap::OptionExt;
use tui_input::backend::crossterm::EventHandler;

use crate::ui::prelude::*;

pub struct Messenger {
    list_state: ListState,
    input_widget: TuiInput,
    hotkeys_component: Hotkeys,
}

impl Messenger {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            input_widget: TuiInput::default(),
            hotkeys_component: Hotkeys::new(vec![
                Hotkey {
                    key: "enter".to_string(),
                    label: "send message".to_string(),
                },
                Hotkey {
                    key: "↑↓".to_string(),
                    label: "scroll".to_string(),
                },
                Hotkey {
                    key: "esc".to_string(),
                    label: "switch channel".to_string(),
                },
            ]),
        }
    }
}

impl Component for Messenger {
    fn handle_event(&mut self, _state: &State, event: &Event, emit: &impl Fn(AppEvent)) {
        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Up => self.list_state.previous(),
                KeyCode::Down => self.list_state.next(),
                KeyCode::Esc => emit(AppEvent::SwitchChannelRequested),
                _ => {
                    self.input_widget.handle_event(event);
                }
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
        let v = ratatui::layout::Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(area);

        let unknown_node = Node::unknown();
        let empty_messages_vec: VecDeque<Message> = VecDeque::default();

        let messages = state
            .messages
            .get(&state.active_channel_key.unwrap_or_log())
            .unwrap_or(&empty_messages_vec);

        // list
        let list_builder = ListBuilder::new(|context| {
            let message = &messages[context.index as usize];
            let node = state.nodes.get(&message.from).unwrap_or(&unknown_node);

            let item = MessageWidget {
                node: &node,
                message,
                is_selected: context.is_selected,
            };

            let height = item.height(area.width);

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
            .scrollbar(scrollbar);

        list.render(v[0], frame.buffer_mut(), &mut self.list_state);

        // input
        let input_block = Block::bordered()
            .padding(Padding::symmetric(1, 0))
            .border_type(BorderType::Rounded)
            .border_style(Style::new().dark_gray());

        let input_block_area = input_block.inner(v[1]);

        input_block.render(v[1], frame.buffer_mut());

        let input_width = input_block_area.width.max(1);
        let scroll = self.input_widget.visual_scroll(input_width as usize);

        let input = Paragraph::new(if !self.input_widget.value().is_empty() {
            Span::from(self.input_widget.value())
        } else {
            Span::from("type message...".to_owned()).dark_gray()
        })
        .scroll((0, scroll as u16));

        frame.render_widget(input, input_block_area);

        let x = self.input_widget.visual_cursor().max(scroll) - scroll;
        frame.set_cursor_position((input_block_area.x + x as u16, input_block_area.y));

        self.hotkeys_component.render(state, frame, v[2]);
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

        text_height + 2
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
            y: area.y + 1,
            width: area.width - 2,
            height: text_height + 1,
        };

        let block = Block::bordered()
            .borders(Borders::LEFT)
            .border_type(BorderType::Thick)
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
            .constraints([Constraint::Length(1), Constraint::Length(text_height)])
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
            Span::from(" "),
            Span::from(self.node.long_name.clone()).bold(),
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
    }
}
