use crate::ui::prelude::*;
use hostaddr::HostAddr;
use itertools::Itertools;
use meshtastic::protobufs::HardwareModel;

pub struct Connection<'a> {
    list_state: ListState,
    discovery_list_state: ListState,
    popup_input_state: PopupInputState<'a>,
    renaming_device: Option<Device>,
    removing_device: Option<Device>,
    is_discovery_popup_visible: bool,
    is_tcp_form_visible: bool,
}

impl<'a> Connection<'a> {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            discovery_list_state: ListState::default(),
            popup_input_state: PopupInputState::default(),
            renaming_device: None,
            removing_device: None,
            is_discovery_popup_visible: false,
            is_tcp_form_visible: false,
        }
    }

    fn render_discovery_popup(&mut self, area: Rect, buf: &mut Buffer, state: &State) {
        let block = Block::bordered()
            .border_type(BorderType::Thick)
            .padding(Padding::new(1, 1, 1, 0))
            .title(Line::from(vec![
                Span::from(" device discovery "),
                Span::from("(").dark_gray(),
                match state.device_discovering_state {
                    DeviceDiscoveringState::NotStarted => Span::from("not started"),
                    DeviceDiscoveringState::Scanning => Span::from("scanning...").yellow(),
                    DeviceDiscoveringState::Finished => Span::from("finished").green(),
                },
                Span::from(") ").dark_gray(),
            ]));

        let block_area = block.inner(area);

        Clear.render(area, buf);
        block.render(area, buf);

        self.discovery_list_state.fix_selection(state.devices_discovered.len());

        if !state.devices_discovered.is_empty() {
            let list_builder = ListBuilder::new(|context| {
                let device = state.devices_discovered.iter().nth(context.index).unwrap();

                let item = DeviceWidget {
                    device,
                    is_selected: context.is_selected,
                    centered: false,
                };

                (item, 1)
            });

            let list = ListView::new(list_builder, state.devices_discovered.len())
                .infinite_scrolling(false)
                .scrollbar(default_scrollbar());

            list.render(block_area, buf, &mut self.discovery_list_state);
        } else {
            match &state.device_discovering_state {
                DeviceDiscoveringState::NotStarted => PlaceholderWidget::dark_gray("nothing to show"),
                DeviceDiscoveringState::Scanning => PlaceholderWidget::yellow("scanning..."),
                DeviceDiscoveringState::Finished => PlaceholderWidget::dark_gray("devices not found"),
            }
            .render(block_area, buf);
        }
    }
}

impl<'a> Component for Connection<'a> {
    fn get_hotkeys(&self, state: &State) -> Vec<Hotkey> {
        if state.active_device.is_some() {
            return vec![Hotkey::new("esc", "disconnect")];
        }

        if self.is_tcp_form_visible {
            return vec![Hotkey::new("enter", "submit"), Hotkey::new("esc", "cancel")];
        }

        if self.renaming_device.is_some() {
            return vec![Hotkey::new("enter", "submit"), Hotkey::new("esc", "cancel")];
        }

        if self.is_discovery_popup_visible {
            return vec![Hotkey::new("enter", "select"), Hotkey::new("esc", "close")];
        }

        let is_list_selected = self.list_state.selected.is_some();

        vec![
            Some(Hotkey::new("↑↓", "scroll")),
            Some(Hotkey::new("enter", "connect")),
            Some(Hotkey::new("t", "add TCP")),
            Some(Hotkey::new("d", "discover")),
            is_list_selected.then_some(Hotkey::new("r", "rename")),
            is_list_selected.then_some(Hotkey::new("delete", "remove")),
        ]
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
        if let Some(removing_device) = self.removing_device.as_ref() {
            match event {
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    modifiers,
                    ..
                }) if modifiers.is_empty() => match code {
                    KeyCode::Enter => {
                        emit(AppEvent::DeviceRemoveRequested(removing_device.clone()))?;
                        self.removing_device = None;
                    }
                    KeyCode::Esc => {
                        self.removing_device = None;
                    }
                    _ => {}
                },
                _ => {}
            }

            return Ok(true);
        }

        if self.is_discovery_popup_visible {
            if self
                .discovery_list_state
                .handle_navigation_events(event, Some(state.devices_discovered.len()))
            {
                return Ok(true);
            }

            match event {
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    modifiers,
                    ..
                }) if modifiers.is_empty() => match code {
                    KeyCode::Enter => {
                        if let Some(device) = self
                            .discovery_list_state
                            .selected
                            .and_then(|i| state.devices_discovered.iter().nth(i))
                        {
                            emit(AppEvent::DeviceSubmitted(device.clone()))?
                        }

                        self.is_discovery_popup_visible = false;

                        return Ok(true);
                    }
                    KeyCode::Esc => {
                        self.is_discovery_popup_visible = false;
                        return Ok(true);
                    }
                    _ => {}
                },
                _ => {}
            }

            return Ok(false);
        }

        if self.is_tcp_form_visible {
            match event {
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    modifiers,
                    ..
                }) if modifiers.is_empty() => match code {
                    KeyCode::Enter => match self.popup_input_state.get_value().parse::<HostAddr<String>>() {
                        Ok(address) => {
                            emit(AppEvent::DeviceSubmitted(Device::Tcp { address, name: None }))?;
                            self.is_tcp_form_visible = false;
                        }
                        Err(e) => {
                            self.popup_input_state.set_error(format!("invalid address: {}", e));
                        }
                    },
                    KeyCode::Esc => {
                        self.is_tcp_form_visible = false;
                    }
                    _ => {}
                },
                Event::Paste(text) => {
                    self.popup_input_state.insert_str(text);
                }
                _ => {}
            }

            let _ = self.popup_input_state.handle_event(event.clone());

            return Ok(true);
        }

        if let Some(device) = self.renaming_device.as_ref() {
            match event {
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    modifiers,
                    ..
                }) if modifiers.is_empty() => match code {
                    KeyCode::Enter => {
                        let value = self.popup_input_state.get_value().trim().to_owned();

                        if value.len() > 32 {
                            self.popup_input_state.set_error("max length is 32");
                            return Ok(true);
                        }

                        if value.is_empty() {
                            emit(AppEvent::DeviceSubmitted(device.without_name()))?;
                        } else {
                            emit(AppEvent::DeviceSubmitted(device.with_name(value.to_owned())))?;
                        }

                        self.renaming_device = None;
                    }
                    KeyCode::Esc => {
                        self.renaming_device = None;
                    }
                    _ => {}
                },
                Event::Paste(text) => {
                    self.popup_input_state.insert_str(text);
                }
                _ => {}
            }

            let _ = self.popup_input_state.handle_event(event.clone());

            return Ok(true);
        }

        if state.active_device.is_some() {
            match event {
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    modifiers,
                    ..
                }) => match code {
                    KeyCode::Esc if modifiers.is_empty() => {
                        emit(AppEvent::DisconnectionRequested)?;
                        return Ok(true);
                    }
                    KeyCode::Tab | KeyCode::BackTab => {
                        return Ok(false);
                    }
                    _ => {}
                },
                _ => {}
            }

            return Ok(false);
        }

        if self
            .list_state
            .handle_navigation_events(event, Some(state.devices.len()))
        {
            return Ok(true);
        }

        match event {
            Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                modifiers,
                ..
            }) => match code {
                KeyCode::Char('d') if modifiers.is_empty() => {
                    self.is_discovery_popup_visible = true;
                    emit(AppEvent::DeviceRediscoverRequested)?;

                    return Ok(true);
                }
                KeyCode::Char('t') if modifiers.is_empty() => {
                    self.popup_input_state.set_title(Some(" new TCP device "));
                    self.popup_input_state.set_placeholder("host[:port=4403]");
                    self.popup_input_state.reset();

                    self.is_tcp_form_visible = true;

                    return Ok(true);
                }
                KeyCode::Char('r') if modifiers.is_empty() => {
                    if let Some(device) = self.list_state.selected.and_then(|i| state.devices.iter().nth(i)) {
                        self.popup_input_state.set_title(Some(" rename device "));
                        self.popup_input_state.set_placeholder("no name");
                        self.popup_input_state.reset();
                        self.popup_input_state.insert_str(device.name().unwrap_or(""));

                        self.renaming_device = Some(device.clone());
                    }
                    return Ok(true);
                }
                KeyCode::Enter if modifiers.is_empty() => {
                    if let Some(device) = self.list_state.selected.and_then(|i| state.devices.iter().nth(i)) {
                        emit(AppEvent::DeviceSelected(device.clone()))?
                    }
                    return Ok(true);
                }
                KeyCode::Delete | KeyCode::Backspace if modifiers.is_empty() => {
                    if let Some(device) = self.list_state.selected.and_then(|i| state.devices.iter().nth(i)) {
                        self.removing_device = Some(device.clone());
                    }
                    return Ok(true);
                }
                _ => {}
            },
            _ => {}
        }

        Ok(false)
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        self.list_state.fix_selection(state.devices.len());

        if let Some(active) = &state.active_device {
            self.list_state.select(state.devices.iter().position(|d| active == d));
        }

        if !state.devices.is_empty() {
            let list_builder = ListBuilder::new(|context| {
                let device = state.devices.iter().nth(context.index).unwrap();

                let item = DeviceWidget {
                    device,
                    is_selected: context.is_selected,
                    centered: false,
                };

                (item, 1)
            });

            let list = ListView::new(list_builder, state.devices.len())
                .infinite_scrolling(false)
                .scrollbar(default_scrollbar())
                .add_modifier(if state.active_device.is_some() {
                    Modifier::DIM
                } else {
                    Modifier::empty()
                });

            list.render(area, frame.buffer_mut(), &mut self.list_state);
        } else {
            PlaceholderWidget::dark_gray("no devices").render(area, frame.buffer_mut());
        }

        if self.is_discovery_popup_visible {
            self.render_discovery_popup(
                area.centered(Constraint::Length(44), Constraint::Length(12)),
                frame.buffer_mut(),
                state,
            );
        }

        if self.is_tcp_form_visible || self.renaming_device.is_some() {
            PopupInputWidget::new(36).render(area, frame.buffer_mut(), &mut self.popup_input_state);
        }

        if self.removing_device.is_some() {
            PopupConfirmWidget::new(
                "Are you sure to remove this device?",
                "confirm",
                "cancel",
                40,
                Color::Red,
            )
            .render(area, frame.buffer_mut());
        }

        if let Some(active_device) = &state.active_device {
            let popup_area = Rect {
                x: area.x,
                y: area.y + area.height / 3,
                width: area.width,
                height: area.height - area.height / 3,
            };

            let popup_block = Block::bordered()
                .border_type(BorderType::Thick)
                .padding(Padding::uniform(1))
                .title(" selected connection ");

            let popup_block_area = popup_block.inner(popup_area);

            frame.render_widget(Clear, popup_area);
            frame.render_widget(popup_block, popup_area);

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
                ConnectionState::LoadingConfig => {
                    vec![Line::from(
                        Span::from(format!(
                            "loading nodes {}/{}...",
                            state.nodes_stash.len(),
                            state.nodes_stash_cap
                        ))
                        .yellow(),
                    )]
                }
                ConnectionState::Connected => vec![
                    Some(Line::from(Span::from("connected").green())),
                    state.device_metadata.as_ref().map(|_| Line::from("")),
                    state.device_metadata.as_ref().map(|metadata| Line::from(vec![
                            Span::from(
                                HardwareModel::try_from(metadata.hw_model)
                                    .ok().map(|h| h.as_str_name())
                                    .unwrap_or("UNKNOWN"),
                            )
                            .magenta(),
                            Span::from(format!(" [v{}]", &metadata.firmware_version)).dark_gray(),
                        ])),
                ]
                .into_iter()
                .flatten()
                .collect(),
            };

            let block_v = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(conn_info.len() as u16),
                Constraint::Fill(1),
            ])
            .split(popup_block_area);

            let device_widget = DeviceWidget {
                device: active_device,
                is_selected: false,
                centered: true,
            };

            device_widget.render(block_v[1], frame.buffer_mut());

            frame.render_widget(
                Paragraph::new(conn_info)
                    .alignment(HorizontalAlignment::Center)
                    .wrap(Wrap { trim: false }),
                block_v[3],
            );
        }
    }
}

struct DeviceWidget<'a> {
    device: &'a Device,
    is_selected: bool,
    centered: bool,
}

impl<'a> Widget for DeviceWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let modifier = if self.is_selected {
            Modifier::REVERSED
        } else {
            Modifier::empty()
        };

        let spans = match self.device {
            Device::Ble { name, address, id } => vec![
                Span::from(" BLE ").black().on_blue(),
                Span::from(format!(
                    " {} ",
                    [name, id, &Some(address.to_string())].into_iter().flatten().join(" – ")
                ))
                .add_modifier(modifier),
            ],
            Device::Tcp { name, address } => vec![
                Span::from(" TCP ").black().on_green(),
                Span::from(format!(
                    " {} ",
                    [name, &Some(address.to_string())].into_iter().flatten().join(" – ")
                ))
                .add_modifier(modifier),
            ],
            Device::Serial { name, address } => vec![
                Span::from(" COM ").black().on_magenta(),
                Span::from(format!(
                    " {} ",
                    [name, &Some(address.to_string())].into_iter().flatten().join(" – ")
                ))
                .add_modifier(modifier),
            ],
        };

        let mut line = Line::from(spans);

        if self.centered {
            line = line.centered();
        }

        line.render(area, buf);
    }
}
