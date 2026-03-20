use tracing_unwrap::OptionExt;

use crate::ui::prelude::*;

pub struct DeviceSelected {
    hotkeys_component: Hotkeys,
}

impl DeviceSelected {
    pub fn new() -> Self {
        Self {
            hotkeys_component: Hotkeys::new(vec![Hotkey {
                key: "esc".to_string(),
                label: "disconnect".to_string(),
            }]),
        }
    }
}

impl Component for DeviceSelected {
    fn handle_event(&mut self, _state: &State, event: &Event, emit: &impl Fn(AppEvent)) {
        if let Event::Key(KeyEvent { code, .. }) = event {
            match code {
                KeyCode::Esc => emit(AppEvent::DisconnectionRequested),
                _ => {}
            };
        }
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        let v = ratatui::layout::Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        let block = Block::bordered()
            .title(" connection ".to_string())
            .border_type(ratatui::widgets::BorderType::Rounded)
            .padding(Padding::uniform(1))
            .dark_gray();

        let block_area = block.inner(v[0]);

        frame.render_widget(block, v[0]);

        let block_v = ratatui::layout::Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(2),
                Constraint::Max(4),
                Constraint::Min(1),
            ])
            .split(block_area);

        let selected_device = state.app_config.selected_device.clone().unwrap_or_log();

        frame.render_widget(
            Line::from(match &selected_device {
                Device::Ble { name, .. } => {
                    vec![
                        Span::from("BLE ".to_string()).magenta(),
                        Span::from(name).white(),
                    ]
                }
                Device::Tcp(hostaddr) => {
                    vec![
                        Span::from("TCP ".to_string()).magenta(),
                        Span::from(hostaddr.to_string()).white(),
                    ]
                }
                Device::Serial(address) => {
                    vec![
                        Span::from("COM ".to_string()).magenta(),
                        Span::from(address).white(),
                    ]
                }
            })
            .alignment(HorizontalAlignment::Center),
            block_v[1],
        );

        let conn_info: Vec<Line> = match &state.connection_state {
            ConnectionState::NotConnected => {
                vec![Line::from(Span::from(" not connected ").dark_gray())]
            }
            ConnectionState::ProblemDetected { error, .. } => vec![
                Line::from(Span::from(" connection problem ").white().on_red()),
                Line::from(""),
                Line::from(Span::from(error).red()),
            ],
            ConnectionState::Connecting => vec![Line::from(
                Span::from(" connecting... ").black().on_yellow(),
            )],
            ConnectionState::Connected => {
                vec![Line::from(Span::from(" connected ").white().on_green())]
            }
        };

        frame.render_widget(
            Paragraph::new(conn_info)
                .alignment(HorizontalAlignment::Center)
                .wrap(Wrap { trim: false }),
            block_v[2],
        );

        self.hotkeys_component.render(state, frame, v[1]);
    }
}
