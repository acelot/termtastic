use chrono::Local;
use tracing::Level;

use crate::ui::prelude::*;

pub struct Logs {
    list_state: ListState,
    follow: bool,
    selected_id: Option<u64>,
    popup_record: Option<LogRecord>,
    popup_scroll_state: ScrollbarState,
}

impl Logs {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            follow: true,
            selected_id: None,
            popup_record: None,
            popup_scroll_state: ScrollbarState::default(),
        }
    }

    fn sync_selection(&mut self, state: &State) {
        if self.follow {
            self.selected_id = state.logs.back().map(|r| r.id);
        }

        self.list_state.selected = match (self.selected_id, state.logs.front()) {
            (Some(id), Some(front)) => Some(id.saturating_sub(front.id) as usize),
            _ => None,
        };
    }
}

impl Component for Logs {
    fn get_hotkeys(&self, _state: &State) -> Vec<Hotkey> {
        if self.popup_record.is_some() {
            return vec![
                Hotkey::new("↑↓".to_string(), "scroll".to_string()),
                Hotkey::new("c".to_string(), "copy".to_string()),
                Hotkey::new("esc".to_string(), "close".to_string()),
            ];
        }

        vec![
            Hotkey::new("↑↓".to_string(), "scroll".to_string()),
            Hotkey::new("enter".to_string(), "expand".to_string()),
            Hotkey::new("c".to_string(), "copy".to_string()),
            Hotkey::new("home".to_string(), "to top".to_string()),
            Hotkey::new("end".to_string(), "to bottom".to_string()),
        ]
    }

    fn handle_event(
        &mut self,
        state: &State,
        event: &Event,
        emit: &impl Fn(AppEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<bool> {
        self.sync_selection(state);

        if let Some(record) = &self.popup_record {
            match event {
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    modifiers,
                    ..
                }) if modifiers.is_empty() => match code {
                    KeyCode::Up => {
                        self.popup_scroll_state.prev();
                        return Ok(true);
                    }
                    KeyCode::Down => {
                        self.popup_scroll_state.next();
                        return Ok(true);
                    }
                    KeyCode::Char('c') => {
                        emit(AppEvent::CopyToClipboardRequested(record.clone().into()))?;
                        return Ok(true);
                    }
                    KeyCode::Esc => {
                        self.popup_record = None;
                        return Ok(true);
                    }
                    _ => {}
                },
                Event::Mouse(MouseEvent { kind, .. }) => match kind {
                    MouseEventKind::ScrollUp => {
                        self.popup_scroll_state.prev();
                        return Ok(true);
                    }
                    MouseEventKind::ScrollDown => {
                        self.popup_scroll_state.next();
                        return Ok(true);
                    }
                    _ => {}
                },
                _ => {}
            }

            return Ok(false);
        }

        if self.list_state.handle_navigation_events(event, Some(state.logs.len())) {
            if let Some(index) = self.list_state.selected {
                self.selected_id = Some(state.logs[index].id);
                self.follow = index == state.logs.len() - 1;
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
                KeyCode::Enter if modifiers.is_empty() => {
                    if let Some(i) = self.list_state.selected {
                        self.popup_record = Some(state.logs[i].clone());
                        self.popup_scroll_state.first();
                    }

                    return Ok(true);
                }
                KeyCode::Char('c')
                    if modifiers.is_empty()
                        && let Some(i) = self.list_state.selected =>
                {
                    emit(AppEvent::CopyToClipboardRequested(state.logs[i].clone().into()))?;
                    return Ok(true);
                }
                _ => {}
            },
            _ => {}
        }

        Ok(false)
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        self.sync_selection(state);

        let v = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).split(area);

        if !state.logs.is_empty() {
            let list_builder = ListBuilder::new(|context| {
                let item = LogRecordWidget {
                    paragraph: get_record_paragraph(&state.logs[context.index], context.is_selected, false, false),
                };

                (item, 1)
            });

            let list = ListView::new(list_builder, state.logs.len())
                .scrollbar(default_scrollbar())
                .infinite_scrolling(false)
                .add_modifier(if self.popup_record.is_some() {
                    Modifier::DIM
                } else {
                    Modifier::empty()
                });

            list.render(v[0], frame.buffer_mut(), &mut self.list_state);
        } else {
            PlaceholderWidget::dark_gray("no logs yet").render(v[0], frame.buffer_mut());
        }

        if let Some(record) = &self.popup_record {
            let popup_area = Rect {
                x: area.x,
                y: area.y + area.height / 4,
                width: area.width,
                height: area.height - area.height / 4,
            };

            let popup_block = Block::new()
                .title(" expanded view ")
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .border_style(Style::new().white())
                .padding(Padding::symmetric(1, 0));

            let popup_block_area = popup_block.inner(popup_area);

            Clear.render(popup_area.outer(Margin::new(1, 0)), frame.buffer_mut());
            popup_block.render(popup_area, frame.buffer_mut());

            let paragraph = get_record_paragraph(record, false, true, true);
            let paragraph_lines = paragraph.line_count(popup_block_area.width - 2);
            self.popup_scroll_state = self
                .popup_scroll_state
                .content_length(paragraph_lines.saturating_sub(popup_block_area.height as usize) + 1);

            StatefulWidget::render(
                LogRecordWidget { paragraph },
                popup_block_area,
                frame.buffer_mut(),
                &mut self.popup_scroll_state,
            );
        }
    }
}

struct LogRecordWidget<'a> {
    pub paragraph: Paragraph<'a>,
}

impl<'a> Widget for LogRecordWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = Rect::new(area.x, area.y, area.width - 2, area.height);
        self.paragraph.render(area, buf);
    }
}

impl<'a> StatefulWidget for LogRecordWidget<'a> {
    type State = ScrollbarState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State)
    where
        Self: Sized,
    {
        let h = Layout::horizontal([Constraint::Fill(1), Constraint::Length(1), Constraint::Length(1)]).split(area);

        self.paragraph
            .scroll((state.get_position() as u16, 0))
            .render(h[0], buf);

        default_scrollbar().render(h[2], buf, state);
    }
}

fn get_record_paragraph(record: &'_ LogRecord, is_selected: bool, with_source: bool, wrap: bool) -> Paragraph<'_> {
    Paragraph::new(Line::from(
        vec![
            Some(Span::from(record.datetime.with_timezone(&Local).format("%H:%M:%S").to_string()).dark_gray()),
            Some(
                Span::from(format!("{:^7}", record.level.to_string())).style(match record.level {
                    Level::TRACE | Level::DEBUG => Style::default().green(),
                    Level::INFO => Style::default().blue(),
                    Level::WARN => Style::default().yellow(),
                    Level::ERROR => Style::default().red(),
                }),
            ),
            with_source.then_some(Span::from(format!("[{}]", &record.source).magenta())),
            with_source.then_some(Span::from(" ")),
            Some(Span::from(&record.message)),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<Span<'_>>>(),
    ))
    .add_modifier(if is_selected {
        Modifier::REVERSED
    } else {
        Modifier::empty()
    })
    .wrap(Wrap { trim: !wrap })
}
