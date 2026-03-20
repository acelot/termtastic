use arboard::Clipboard;
use tracing::Level;

use crate::ui::prelude::*;

pub struct Logs {
    list_state: tui_widget_list::ListState,
    follow: bool,
    popup_record: Option<LogRecord>,
    popup_scroll_offset: u16,
    hotkeys_component: Hotkeys,
    popup_hotkeys_component: Hotkeys,
}

impl Logs {
    pub fn new() -> Self {
        Self {
            list_state: tui_widget_list::ListState::default(),
            follow: true,
            popup_record: None,
            popup_scroll_offset: 0,
            hotkeys_component: Hotkeys::new(vec![
                Hotkey {
                    key: "\u{2191}\u{2193}".to_string(),
                    label: "navigate".to_string(),
                },
                Hotkey {
                    key: "enter".to_string(),
                    label: "expand".to_string(),
                },
                Hotkey {
                    key: "c".to_string(),
                    label: "copy".to_string(),
                },
                Hotkey {
                    key: "home".to_string(),
                    label: "go first".to_string(),
                },
                Hotkey {
                    key: "end".to_string(),
                    label: "go last".to_string(),
                },
                Hotkey {
                    key: "f".to_string(),
                    label: "follow".to_string(),
                },
            ]),
            popup_hotkeys_component: Hotkeys::new(vec![
                Hotkey {
                    key: "\u{2191}\u{2193}".to_string(),
                    label: "scroll".to_string(),
                },
                Hotkey {
                    key: "c".to_string(),
                    label: "copy".to_string(),
                },
                Hotkey {
                    key: "esc".to_string(),
                    label: "close".to_string(),
                },
            ]),
        }
    }

    fn copy_to_clipboard(&self, record: &LogRecord) {
        let mut clipboard = Clipboard::new().unwrap();

        clipboard
            .set_text(format!(
                "{} {} {}: {}",
                record.datetime.to_rfc3339(),
                record.level.to_string(),
                record.source.clone(),
                record.message
            ))
            .unwrap();
    }
}

impl Component for Logs {
    fn handle_event(&mut self, state: &State, event: &Event, _emit: &impl Fn(AppEvent)) {
        if let Event::Key(KeyEvent { code, .. }) = event {
            match (code, self.popup_record.is_some()) {
                (KeyCode::Up, false) => {
                    self.follow = false;
                    self.list_state.previous();
                }
                (KeyCode::Down, false) => {
                    self.follow = false;
                    self.list_state.next();
                }
                (KeyCode::Enter, false) => {
                    if let Some(i) = self.list_state.selected {
                        self.popup_record = Some(state.logs[i].clone());
                        self.popup_scroll_offset = 0;
                    }
                }
                (KeyCode::Home, false) => {
                    self.list_state.select(Some(0));
                }
                (KeyCode::End, false) => {
                    self.list_state.select(Some(state.logs.len() - 1));
                }
                (KeyCode::Char('f'), false) => {
                    self.follow = true;
                }
                // popup hotkeys
                (KeyCode::Up, true) => {
                    self.popup_scroll_offset = self.popup_scroll_offset.saturating_sub(1);
                }
                (KeyCode::Down, true) => {
                    self.popup_scroll_offset = self.popup_scroll_offset.saturating_add(1);
                }
                (KeyCode::Esc, true) => {
                    self.popup_record = None;
                }
                // general
                (KeyCode::Char('c'), _) => {
                    if let Some(i) = self.list_state.selected {
                        self.copy_to_clipboard(&state.logs[i]);
                    }
                }
                _ => {}
            };
        }
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        if self.follow {
            self.list_state.select(Some(state.logs.len() - 1));
        }

        let v = ratatui::layout::Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        let list_builder = tui_widget_list::ListBuilder::new(|context| {
            let item = get_record_widget(
                state.logs[context.index].clone(),
                if context.is_selected {
                    Span::from("> ").yellow()
                } else {
                    Span::from("  ")
                },
            );

            (item, 1)
        });

        let list = tui_widget_list::ListView::new(list_builder, state.logs.len()).block(
            Block::new()
                .borders(Borders::LEFT)
                .border_style(Style::new().dark_gray()),
        );

        list.render(v[0], frame.buffer_mut(), &mut self.list_state);

        if let Some(r) = &self.popup_record {
            let popup_area = Rect {
                x: v[0].x,
                y: v[0].y + v[0].height / 4,
                width: v[0].width,
                height: v[0].height - v[0].height / 4,
            };

            let popup = Paragraph::new(get_record_widget(r.clone(), Span::from("")))
                .wrap(Wrap { trim: false })
                .scroll((self.popup_scroll_offset, 0))
                .block(
                    Block::new()
                        .title(" expanded view ")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::new().dark_gray())
                        .padding(Padding::uniform(1)),
                );

            frame.render_widget(Clear, popup_area);
            frame.render_widget(popup, popup_area);
        }

        if self.popup_record.is_some() {
            self.popup_hotkeys_component.render(state, frame, v[2]);
        } else {
            self.hotkeys_component.render(state, frame, v[2]);
        }
    }
}

fn get_record_widget(record: LogRecord, first_span: Span) -> Line {
    Line::from(vec![
        first_span,
        Span::from(record.datetime.format("%H:%M:%S").to_string()).dark_gray(),
        Span::from(" "),
        Span::from(format!("{:<5}", record.level.to_string())).style(match record.level {
            Level::TRACE | Level::DEBUG => Style::default().green(),
            Level::INFO => Style::default().blue(),
            Level::WARN => Style::default().yellow(),
            Level::ERROR => Style::default().red(),
        }),
        Span::from(" "),
        Span::from(format!("{}: ", record.source)).dark_gray(),
        Span::from(record.message.clone()).white(),
    ])
}
