use arboard::Clipboard;
use chrono::Local;
use tracing::Level;

use crate::ui::prelude::*;

pub struct Logs {
    list_state: ListState,
    follow: bool,
    popup_record: Option<LogRecord>,
    popup_scroll_offset: u16,
    hotkeys_component: Hotkeys,
    popup_hotkeys_component: Hotkeys,
}

impl Logs {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            follow: true,
            popup_record: None,
            popup_scroll_offset: 0,
            hotkeys_component: Hotkeys::new(vec![
                Hotkey {
                    key: "↑↓".to_string(),
                    label: "scroll".to_string(),
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
                    label: "to top".to_string(),
                },
                Hotkey {
                    key: "end".to_string(),
                    label: "to bottom".to_string(),
                },
            ]),
            popup_hotkeys_component: Hotkeys::new(vec![
                Hotkey {
                    key: "↑↓".to_string(),
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
    fn handle_event(
        &mut self,
        state: &State,
        event: &Event,
        _emit: &impl Fn(AppEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        match event {
            Event::Key(KeyEvent { code, .. }) => match (code, self.popup_record.is_some()) {
                (KeyCode::Up, false) => {
                    self.follow = false;
                    self.list_state.previous();
                }
                (KeyCode::Down, false) => {
                    self.list_state.next();

                    if let Some(index) = self.list_state.selected {
                        self.follow = index == state.logs.len() - 1;
                    }
                }
                (KeyCode::Enter, false) => {
                    if let Some(i) = self.list_state.selected {
                        self.popup_record = Some(state.logs[i].clone());
                        self.popup_scroll_offset = 0;
                    }
                }
                (KeyCode::Home, false) => {
                    self.follow = false;
                    self.list_state.select(Some(0));
                }
                (KeyCode::End, false) => {
                    self.follow = true;
                    self.list_state.select(Some(state.logs.len() - 1));
                }
                // popup hotkeys
                (KeyCode::Up, true) => {
                    self.popup_scroll_offset = self.popup_scroll_offset.saturating_sub(1)
                }
                (KeyCode::Down, true) => {
                    self.popup_scroll_offset = self.popup_scroll_offset.saturating_add(1);
                }
                (KeyCode::Esc, true) => self.popup_record = None,
                // general
                (KeyCode::Char('c'), _) => {
                    if let Some(i) = self.list_state.selected {
                        self.copy_to_clipboard(&state.logs[i]);
                    }
                }
                _ => {}
            },
            Event::Mouse(MouseEvent { kind, .. }) => match kind {
                MouseEventKind::ScrollUp => {
                    self.follow = false;
                    self.list_state.previous();
                }
                MouseEventKind::ScrollDown => {
                    self.list_state.next();

                    if let Some(index) = self.list_state.selected {
                        self.follow = index == state.logs.len() - 1;
                    }
                }
                _ => {}
            },
            _ => {}
        }

        Ok(())
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        if self.follow && !state.logs.is_empty() {
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

        let list_builder = ListBuilder::new(|context| {
            let item = LogRecordWidget {
                record: &state.logs[context.index],
                is_selected: context.is_selected,
                wrap: false,
                scroll_offset: 0,
            };

            (item, 1)
        });

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .symbols(ScrollbarSet {
                begin: "┬",
                thumb: "█",
                track: "│",
                end: "┴",
            })
            .style(Style::new().dark_gray());

        let list = ListView::new(list_builder, state.logs.len())
            .scrollbar(scrollbar)
            .infinite_scrolling(false);

        list.render(v[0], frame.buffer_mut(), &mut self.list_state);

        if let Some(r) = &self.popup_record {
            let popup_area = Rect {
                x: v[0].x,
                y: v[0].y + v[0].height / 4,
                width: v[0].width,
                height: v[0].height - v[0].height / 4,
            };

            let popup_block = Block::new()
                .title(" expanded view ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::new().white())
                .padding(Padding::uniform(1));

            let popup_block_area = popup_block.inner(popup_area);

            Clear.render(popup_area, frame.buffer_mut());
            popup_block.render(popup_area, frame.buffer_mut());

            LogRecordWidget {
                record: r,
                is_selected: false,
                wrap: true,
                scroll_offset: self.popup_scroll_offset,
            }
            .render(popup_block_area, frame.buffer_mut());
        }

        if self.popup_record.is_some() {
            self.popup_hotkeys_component.render(state, frame, v[2]);
        } else {
            self.hotkeys_component.render(state, frame, v[2]);
        }
    }
}

struct LogRecordWidget<'a> {
    pub record: &'a LogRecord,
    pub is_selected: bool,
    pub wrap: bool,
    pub scroll_offset: u16,
}

impl<'a> Widget for LogRecordWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let area = area.resize(Size {
            width: area.width - 2,
            height: area.height,
        });

        let tz = Local;

        let mut p = Paragraph::new(Line::from(vec![
            Span::from(
                self.record
                    .datetime
                    .with_timezone(&tz)
                    .format("%H:%M:%S")
                    .to_string(),
            )
            .dark_gray(),
            Span::from(" ").dark_gray(),
            Span::from(format!("{:<5}", self.record.level.to_string())).style(
                match self.record.level {
                    Level::TRACE | Level::DEBUG => Style::default().green(),
                    Level::INFO => Style::default().blue(),
                    Level::WARN => Style::default().yellow(),
                    Level::ERROR => Style::default().red(),
                },
            ),
            Span::from(" ").dark_gray(),
            Span::from(format!("{}: ", self.record.source)).dark_gray(),
            Span::from(self.record.message.clone()),
        ]))
        .scroll((self.scroll_offset, 0));

        if self.is_selected {
            p = p.reversed();
        }

        if self.wrap {
            p = p.wrap(Wrap { trim: false });
        }

        p.render(area, buf);
    }
}
