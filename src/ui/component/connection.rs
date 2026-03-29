use hostaddr::HostAddr;
use itertools::Itertools;
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::ui::prelude::*;

pub struct Connection {
    devices: Vec<Device>,
    list_state: ListState,
    is_form_visible: bool,
    form_error: Option<String>,
    form_input: Input,
}

impl Connection {
    pub fn new() -> Self {
        Self {
            devices: vec![],
            list_state: ListState::default(),
            is_form_visible: false,
            form_error: None,
            form_input: Input::default(),
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
                label: "navigate".to_string(),
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

impl Component for Connection {
    fn handle_event(&mut self, state: &State, event: &Event, emit: &impl Fn(AppEvent)) {
        match event {
            Event::Key(KeyEvent { code, .. }) => {
                let is_device_active = state.active_device.is_some();

                match (code, self.is_form_visible, is_device_active) {
                    (KeyCode::Up, false, false) => self.list_state.previous(),
                    (KeyCode::Down, false, false) => self.list_state.next(),
                    (KeyCode::Char('r'), false, false) => emit(AppEvent::DeviceRediscoverRequested),
                    (KeyCode::Char('t'), false, false) => {
                        self.form_error = None;
                        self.is_form_visible = true;
                    }
                    (KeyCode::Enter, false, false) => {
                        if let Some(index) = self.list_state.selected {
                            emit(AppEvent::DeviceSelected(self.devices[index].clone()))
                        }
                    }
                    (KeyCode::Enter, true, false) => {
                        match self.form_input.value().parse::<HostAddr<String>>() {
                            Ok(addr) => {
                                emit(AppEvent::TcpDeviceSubmitted(addr));
                                self.is_form_visible = false;
                            }
                            Err(e) => {
                                self.form_error = Some(format!("invalid address: {}", e));
                            }
                        }
                    }
                    (KeyCode::Delete | KeyCode::Backspace, false, false) => {
                        if let Some(index) = self.list_state.selected
                            && let Device::Tcp(hostaddr) = &self.devices[index]
                        {
                            emit(AppEvent::TcpDeviceRemoved(hostaddr.clone()))
                        }
                    }
                    (KeyCode::Esc, true, false) => {
                        self.form_input.reset();
                        self.is_form_visible = false;
                    }
                    (KeyCode::Esc, false, true) => emit(AppEvent::DisconnectionRequested),
                    (_, true, false) => {
                        self.form_input.handle_event(event);
                    }
                    _ => {}
                };
            }
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
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        self.devices = state
            .tcp_devices
            .iter()
            .map(|h| Device::Tcp(h.clone()))
            .chain(state.discovered_devices.clone())
            .sorted()
            .collect();

        if !self.devices.is_empty() && self.list_state.selected.is_none() {
            self.list_state.select(Some(0));
        }

        let list_title_extra = match state.device_discovering_state {
            DevicesDiscoveringState::NeverStarted | DevicesDiscoveringState::Finished => {
                Span::from("")
            }
            DevicesDiscoveringState::InProgress => Span::from("(loading...) ").yellow(),
            DevicesDiscoveringState::Error(_) => Span::from("(error) ").red(),
        };

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

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .symbols(ScrollbarSet {
                begin: "┬",
                thumb: "█",
                track: "│",
                end: "┴",
            })
            .style(Style::new().dark_gray());

        let list = ListView::new(list_builder, self.devices.len())
            .infinite_scrolling(false)
            .scrollbar(scrollbar);

        list.render(v[0], frame.buffer_mut(), &mut self.list_state);

        if self.is_form_visible {
            let popup_area = Rect {
                x: v[0].x + v[0].width / 4,
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
                .title(" new TCP ");

            let popup_block_area = popup_block.inner(popup_area);

            frame.render_widget(Clear, popup_area);
            frame.render_widget(popup_block, popup_area);

            let input_width = popup_block_area.width.max(1) - 1;
            let input_scroll = self.form_input.visual_scroll(input_width as usize);

            let form_input = Paragraph::new(if !self.form_input.value().is_empty() {
                Span::from(self.form_input.value())
            } else {
                Span::from("host[:port=4403]".to_string()).dark_gray()
            })
            .scroll((0, input_scroll as u16));

            frame.render_widget(form_input, popup_block_area);

            let x = self.form_input.visual_cursor().max(input_scroll) - input_scroll;
            frame.set_cursor_position((popup_block_area.x + x as u16, popup_block_area.y));
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
                .flex(Flex::SpaceAround)
                .direction(Direction::Vertical)
                .constraints([Constraint::Fill(1), Constraint::Fill(1)])
                .split(popup_block_area);

            let device_widget = DeviceWidget {
                device: active_device,
                is_selected: false,
                centered: true,
                dimmed: false,
            };

            device_widget.render(block_v[0], frame.buffer_mut());

            let conn_info: Vec<Line> = match &state.connection_state {
                ConnectionState::NotConnected => {
                    vec![Line::from(Span::from("not connected").dark_gray())]
                }
                ConnectionState::ProblemDetected { error, .. } => vec![
                    Line::from(Span::from("connection problem").red()),
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
                block_v[1],
            );
        }

        Hotkeys::new(self.get_hotkeys(state)).render(state, frame, v[1])
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
                Span::from(" BLE ".to_string())
                    .black()
                    .on_blue()
                    .patch_style(dim_style),
                Span::from(" ".to_string()),
                Span::from(name).patch_style(dim_style),
            ],
            Device::Tcp(hostaddr) => vec![
                Span::from(" TCP ".to_string())
                    .black()
                    .on_green()
                    .patch_style(dim_style),
                Span::from(" ".to_string()),
                Span::from(hostaddr.to_string()).patch_style(dim_style),
            ],
            Device::Serial(address) => vec![
                Span::from(" COM ".to_string())
                    .black()
                    .on_magenta()
                    .patch_style(dim_style),
                Span::from(" ".to_string()),
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
