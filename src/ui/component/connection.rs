use hostaddr::HostAddr;
use itertools::Itertools;

use crate::ui::{helpers::default_scrollbar, prelude::*};

pub struct Connection<'a> {
    devices: Vec<Device>,
    list_state: ListState,
    is_form_visible: bool,
    form_error: Option<String>,
    form_input: TextArea<'a>,
}

impl<'a> Connection<'a> {
    pub fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        textarea.set_placeholder_text("host[:port=4403]");

        Self {
            devices: vec![],
            list_state: ListState::default(),
            is_form_visible: false,
            form_error: None,
            form_input: textarea,
        }
    }

    fn get_hotkeys(&self, state: &State) -> Vec<Hotkey> {
        if state.active_device.is_some() {
            return vec![Hotkey {
                key: "esc".to_string(),
                label: "disconnect".to_string(),
            }];
        }

        if self.is_form_visible {
            return vec![
                Hotkey {
                    key: "enter".to_string(),
                    label: "submit".to_string(),
                },
                Hotkey {
                    key: "esc".to_string(),
                    label: "cancel".to_string(),
                },
            ];
        }

        let mut hotkeys = vec![
            Hotkey {
                key: "↑↓".to_string(),
                label: "scroll".to_string(),
            },
            Hotkey {
                key: "enter".to_string(),
                label: "connect".to_string(),
            },
            Hotkey {
                key: "t".to_string(),
                label: "add TCP".to_string(),
            },
            Hotkey {
                key: "r".to_string(),
                label: "rediscover".to_string(),
            },
        ];

        if let Some(index) = self.list_state.selected
            && let Device::Tcp(_) = &self.devices[index]
        {
            hotkeys.push(Hotkey {
                key: "del".to_string(),
                label: "delete".to_string(),
            });
        }

        hotkeys
    }
}

impl<'a> Component for Connection<'a> {
    fn handle_event(
        &mut self,
        state: &State,
        event: &Event,
        emit: &impl Fn(AppEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<bool> {
        if self.is_form_visible {
            match event {
                Event::Key(KeyEvent { code, kind, .. }) if kind == &KeyEventKind::Press => {
                    match code {
                        KeyCode::Enter => {
                            match self.form_input.lines()[0].parse::<HostAddr<String>>() {
                                Ok(addr) => {
                                    emit(AppEvent::TcpDeviceSubmitted(addr))?;
                                    self.is_form_visible = false;
                                }
                                Err(e) => {
                                    self.form_error = Some(format!("invalid address: {}", e));
                                }
                            }
                        }
                        KeyCode::Esc => {
                            self.is_form_visible = false;
                        }
                        _ => {
                            self.form_input.input(event.clone());
                        }
                    }
                }
                _ => {}
            }

            return Ok(true);
        }

        if state.active_device.is_some() {
            match event {
                Event::Key(KeyEvent { code, .. }) => match code {
                    KeyCode::Esc => emit(AppEvent::DisconnectionRequested)?,
                    _ => {}
                },
                _ => {}
            }

            return Ok(true);
        }

        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Up => self.list_state.previous(),
                KeyCode::Down => self.list_state.next(),
                KeyCode::Char('r') => emit(AppEvent::DeviceRediscoverRequested)?,
                KeyCode::Char('t') => {
                    self.form_error = None;
                    self.is_form_visible = true;
                }
                KeyCode::Enter => {
                    if let Some(index) = self.list_state.selected {
                        emit(AppEvent::DeviceSelected(self.devices[index].clone()))?
                    }
                }
                KeyCode::Delete | KeyCode::Backspace => {
                    if let Some(index) = self.list_state.selected
                        && let Device::Tcp(hostaddr) = &self.devices[index]
                    {
                        emit(AppEvent::TcpDeviceRemoved(hostaddr.clone()))?
                    }
                }
                _ => {}
            },
            Event::Mouse(MouseEvent { kind, .. }) => match kind {
                MouseEventKind::ScrollUp => self.list_state.previous(),
                MouseEventKind::ScrollDown => self.list_state.next(),
                _ => {}
            },
            _ => {}
        }

        Ok(true)
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        let v = ratatui::layout::Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        self.devices = state
            .tcp_devices
            .iter()
            .map(|h| Device::Tcp(h.clone()))
            .chain(state.discovered_devices.clone())
            .sorted()
            .collect();

        if !self.devices.is_empty() {
            if self.list_state.selected.is_none()
                && state.device_discovering_state == DeviceDiscoveringState::Done
                && !self.devices.is_empty()
            {
                if let Some(active) = &state.active_device {
                    self.list_state
                        .select(self.devices.iter().position(|d| active == d));
                } else {
                    self.list_state.select(Some(0));
                }
            }

            let list_builder = ListBuilder::new(|context| {
                let device = self.devices.iter().nth(context.index).unwrap();

                let item = DeviceWidget {
                    device,
                    is_selected: context.is_selected,
                    centered: false,
                    dimmed: state.active_device.is_some(),
                };

                (item, 1)
            });

            let list = ListView::new(list_builder, self.devices.len())
                .infinite_scrolling(false)
                .scrollbar(default_scrollbar());

            list.render(v[0], frame.buffer_mut(), &mut self.list_state);
        } else {
            PlaceholderWidget::dark_gray("no devices").render(v[0], frame.buffer_mut());
        }

        if self.is_form_visible {
            let popup_area = Rect {
                x: v[0].x + v[0].width / 2 - 20,
                y: v[0].y + v[0].height / 2 - 2,
                width: 40,
                height: 3,
            };

            let popup_block = Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(if self.form_error.is_none() {
                    Color::Reset
                } else {
                    Color::Red
                }))
                .padding(Padding::symmetric(1, 0))
                .title(" TCP connection ");

            let popup_block_area = popup_block.inner(popup_area);

            frame.render_widget(Clear, popup_area);
            frame.render_widget(popup_block, popup_area);
            frame.render_widget(&self.form_input, popup_block_area);
        }

        if let Some(active_device) = &state.active_device {
            let popup_area = Rect {
                x: v[0].x,
                y: v[0].y + v[0].height / 3,
                width: v[0].width,
                height: v[0].height - v[0].height / 3,
            };

            let popup_block = Block::bordered()
                .border_type(BorderType::Rounded)
                .padding(Padding::uniform(1))
                .title(" selected connection ");

            let popup_block_area = popup_block.inner(popup_area);

            frame.render_widget(Clear, popup_area);
            frame.render_widget(popup_block, popup_area);

            let block_v = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Fill(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Fill(1),
                ])
                .split(popup_block_area);

            let device_widget = DeviceWidget {
                device: active_device,
                is_selected: false,
                centered: true,
                dimmed: false,
            };

            device_widget.render(block_v[1], frame.buffer_mut());

            let conn_info: Vec<Line> = match &state.connection_state {
                ConnectionState::NotConnected => {
                    vec![Line::from(Span::from("not connected").dark_gray())]
                }
                ConnectionState::ProblemDetected { error, .. } => vec![
                    Line::from(Span::from(" connection problem ").white().on_red()),
                    Line::from(""),
                    Line::from(Span::from(error).dark_gray()),
                ],
                ConnectionState::Connecting => {
                    vec![Line::from(Span::from("connecting...").yellow())]
                }
                ConnectionState::Connected => {
                    vec![Line::from(Span::from("connected").green())]
                }
            };

            frame.render_widget(
                Paragraph::new(conn_info)
                    .alignment(HorizontalAlignment::Center)
                    .wrap(Wrap { trim: false }),
                block_v[3],
            );
        }

        HotkeysWidget::new(&self.get_hotkeys(state)).render(v[1], frame.buffer_mut())
    }
}

struct DeviceWidget<'a> {
    device: &'a Device,
    is_selected: bool,
    centered: bool,
    dimmed: bool,
}

impl<'a> Widget for DeviceWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let dim_style = if self.dimmed {
            Style::new().dark_gray().bg(Color::Reset)
        } else {
            Style::new()
        };

        let spans = match self.device {
            Device::Ble { name, .. } => vec![
                Span::from(" BLE ").black().on_blue().patch_style(dim_style),
                Span::from(" "),
                Span::from(name).patch_style(dim_style),
            ],
            Device::Tcp(hostaddr) => vec![
                Span::from(" TCP ")
                    .black()
                    .on_green()
                    .patch_style(dim_style),
                Span::from(" "),
                Span::from(hostaddr.to_string()).patch_style(dim_style),
            ],
            Device::Serial(address) => vec![
                Span::from(" COM ")
                    .black()
                    .on_magenta()
                    .patch_style(dim_style),
                Span::from(" "),
                Span::from(address).patch_style(dim_style),
            ],
        };

        let mut line = Line::from(spans);

        if self.is_selected && !self.dimmed {
            line = line.reversed();
        }

        if self.centered {
            line = line.centered();
        }

        line.render(area, buf);
    }
}
