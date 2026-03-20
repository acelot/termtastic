use hostaddr::HostAddr;
use itertools::Itertools;
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::ui::prelude::*;

pub struct DeviceList {
    devices: Vec<Device>,
    list_state: ListState,
    is_form_visible: bool,
    form_error: Option<String>,
    form_input: Input,
}

impl DeviceList {
    pub fn new() -> Self {
        Self {
            devices: vec![],
            list_state: ListState::default(),
            is_form_visible: false,
            form_error: None,
            form_input: Input::default(),
        }
    }

    fn get_hotkeys(&self) -> Vec<Hotkey> {
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
                key: "\u{2191}\u{2193}".to_string(),
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
                key: "F5".to_string(),
                label: "refresh".to_string(),
            },
        ];

        if let Some(index) = self.list_state.selected()
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

impl Component for DeviceList {
    fn handle_event(&mut self, _state: &State, event: &Event, emit: &impl Fn(AppEvent)) {
        if let Event::Key(KeyEvent { code, .. }) = event {
            match (code, self.is_form_visible) {
                (KeyCode::Up, false) => self.list_state.select_previous(),
                (KeyCode::Down, false) => self.list_state.select_next(),
                (KeyCode::F(5), false) => emit(AppEvent::DeviceRediscoverRequested),
                (KeyCode::Char('t'), false) => {
                    self.form_error = None;
                    self.is_form_visible = true;
                }
                (KeyCode::Enter, false) => {
                    if let Some(index) = self.list_state.selected() {
                        emit(AppEvent::DeviceSelected(self.devices[index].clone()))
                    }
                }
                (KeyCode::Enter, true) => {
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
                (KeyCode::Delete | KeyCode::Backspace, false) => {
                    if let Some(index) = self.list_state.selected()
                        && let Device::Tcp(hostaddr) = &self.devices[index]
                    {
                        emit(AppEvent::TcpDeviceRemoved(hostaddr.clone()))
                    }
                }
                (KeyCode::Esc, true) => {
                    self.form_input.reset();
                    self.is_form_visible = false;
                }
                (_, true) => {
                    self.form_input.handle_event(event);
                }
                _ => {}
            };
        }
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        let v = ratatui::layout::Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        self.devices = state
            .devices_config
            .tcp_devices
            .iter()
            .map(|h| Device::Tcp(h.clone()))
            .chain(state.discovered_devices.clone())
            .sorted()
            .collect();

        let list_items: Vec<ListItem> = self
            .devices
            .iter()
            .map(|device| {
                let spans = match device {
                    Device::Ble { name, .. } => vec![
                        Span::from("BLE".to_string()).blue(),
                        Span::from("  ".to_string()),
                        Span::from(name).white(),
                    ],
                    Device::Tcp(hostaddr) => vec![
                        Span::from("TCP".to_string()).blue(),
                        Span::from("  ".to_string()),
                        Span::from(hostaddr.to_string()).white(),
                    ],
                    Device::Serial(address) => vec![
                        Span::from("COM".to_string()).blue(),
                        Span::from("  ".to_string()),
                        Span::from(address).white(),
                    ],
                };

                ListItem::new(Line::from(spans))
            })
            .collect();

        if !self.devices.is_empty() && self.list_state.selected().is_none() {
            self.list_state.select_first();
        }

        let list_title_extra = match state.device_discovering_state {
            DevicesDiscoveringState::NeverStarted | DevicesDiscoveringState::Finished => {
                Span::from("")
            }
            DevicesDiscoveringState::InProgress => Span::from("(loading...) ").yellow(),
            DevicesDiscoveringState::Error(_) => Span::from("(error) ").red(),
        };

        let list = List::new(list_items)
            .direction(ListDirection::TopToBottom)
            .highlight_symbol(Span::from("> ").yellow())
            .block(
                Block::bordered()
                    .title(Line::from(vec![
                        Span::from(" select device ".to_string()),
                        list_title_extra,
                    ]))
                    .border_type(BorderType::Rounded)
                    .padding(Padding::uniform(1))
                    .dark_gray(),
            );

        frame.render_stateful_widget(list, v[0], &mut self.list_state);

        if self.is_form_visible {
            let popup_area = Rect {
                x: v[0].x + v[0].width / 4,
                y: v[0].y + v[0].height / 2 - 2,
                width: 40,
                height: 3,
            };

            let popup_block = Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(if self.form_error.is_none() {
                    Style::new()
                } else {
                    Style::new().red()
                })
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

        Hotkeys::new(self.get_hotkeys()).render(state, frame, v[1])
    }
}
