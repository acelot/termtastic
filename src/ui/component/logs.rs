use arboard::Clipboard;
use chrono::Local;
use tracing::Level;

use crate::ui::{helpers::default_scrollbar, prelude::*};

pub struct Logs {
    list_state: ListState,
    follow: bool,
    popup_record: Option<LogRecord>,
    popup_scroll_offset: u16,
    hotkeys: Vec<Hotkey>,
    popup_hotkeys: Vec<Hotkey>,
}

impl Logs {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            follow: true,
            popup_record: None,
            popup_scroll_offset: 0,
            hotkeys: vec![
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
            ],
            popup_hotkeys: vec![
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
            ],
        }
    }

    fn copy_to_clipboard(&self, record: &LogRecord) -> anyhow::Result<()> {
        let mut clipboard = Clipboard::new().unwrap();

        clipboard.set_text(format!(
            "{} {} {}: {}",
            record.datetime.to_rfc3339(),
            record.level.to_string(),
            record.source.clone(),
            record.message
        ))?;

        Ok(())
    }
}

impl Component for Logs {
    fn handle_event(
        &mut self,
        state: &State,
        event: &Event,
        emit: &impl Fn(AppEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<bool> {
        if self.popup_record.is_some() {
            match event {
                Event::Key(KeyEvent { code, .. }) => match code {
                    KeyCode::Up => {
                        self.popup_scroll_offset = self.popup_scroll_offset.saturating_sub(1)
                    }
                    KeyCode::Down => {
                        self.popup_scroll_offset = self.popup_scroll_offset.saturating_add(1);
                    }
                    KeyCode::Char('c') if let Some(i) = self.list_state.selected => {
                        match self.copy_to_clipboard(&state.logs[i]) {
                            Ok(_) => emit(AppEvent::ToastRequested(Toast::normal("copied")))?,
                            Err(e) => {
                                emit(AppEvent::ToastRequested(Toast::error("copy failed")))?;
                                tracing::error!("copy failed: {:?}", e);
                            }
                        }
                    }
                    KeyCode::Esc => self.popup_record = None,
                    _ => {}
                },
                _ => {}
            }

            return Ok(true);
        }

        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Up => {
                    self.follow = false;
                    self.list_state.previous();
                }
                KeyCode::Down => {
                    self.list_state.next();

                    if let Some(index) = self.list_state.selected {
                        self.follow = index == state.logs.len() - 1;
                    }
                }
                KeyCode::Enter => {
                    if let Some(i) = self.list_state.selected {
                        self.popup_record = Some(state.logs[i].clone());
                        self.popup_scroll_offset = 0;
                    }
                }
                KeyCode::Home => {
                    self.follow = false;
                    self.list_state.select(Some(0));
                }
                KeyCode::End => {
                    self.follow = true;
                    self.list_state.select(Some(state.logs.len() - 1));
                }
                KeyCode::Char('c') if let Some(i) = self.list_state.selected => {
                    match self.copy_to_clipboard(&state.logs[i]) {
                        Ok(_) => emit(AppEvent::ToastRequested(Toast::normal("copied")))?,
                        Err(e) => {
                            emit(AppEvent::ToastRequested(Toast::error("copy failed")))?;
                            tracing::error!("copy failed: {:?}", e);
                        }
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

        Ok(true)
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

        if !state.logs.is_empty() {
            let list_builder = ListBuilder::new(|context| {
                let item = LogRecordWidget {
                    record: &state.logs[context.index],
                    is_selected: context.is_selected,
                    wrap: false,
                    scroll_offset: 0,
                };

                (item, 1)
            });

            let list = ListView::new(list_builder, state.logs.len())
                .scrollbar(default_scrollbar())
                .infinite_scrolling(false);

            list.render(v[0], frame.buffer_mut(), &mut self.list_state);
        } else {
            PlaceholderWidget::dark_gray("no logs yet").render(v[0], frame.buffer_mut());
        }

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
                .padding(Padding::symmetric(1, 0));

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
            HotkeysWidget::new(&self.popup_hotkeys).render(v[2], frame.buffer_mut());
        } else {
            HotkeysWidget::new(&self.hotkeys).render(v[2], frame.buffer_mut());
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
