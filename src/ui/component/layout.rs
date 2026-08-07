use crate::ui::{
    component::{Chat, Connection, Header, Logs, Nodes, Settings},
    logo::APP_LOGO_TEXT,
    prelude::*,
};
use crossterm::event::KeyModifiers;
use itertools::Itertools;
use ratatui::layout::Layout as RatatuiLayout;
use std::ops::Sub;
use std::time::{Duration, Instant};
use strum::IntoEnumIterator;

const MIN_TERMINAL_SIZE: (u16, u16) = (80, 24);

pub struct Layout<'a> {
    header_component: Header,
    chat_component: Chat<'a>,
    nodes_component: Nodes<'a>,
    settings_component: Settings<'a>,
    connection_component: Connection<'a>,
    logs_component: Logs,
    logo: Text<'static>,
    nodeinfo_widget_state: NodeInfoWidgetState,
    last_esc_t: Instant,
    last_tab: Tab,
}

impl<'a> Layout<'a> {
    pub fn new() -> Self {
        Self {
            header_component: Header::new(),
            chat_component: Chat::new(),
            nodes_component: Nodes::new(),
            settings_component: Settings::new(),
            connection_component: Connection::new(),
            logs_component: Logs::new(),
            logo: APP_LOGO_TEXT.clone(),
            nodeinfo_widget_state: NodeInfoWidgetState::default(),
            last_esc_t: Instant::now().sub(Duration::from_secs(1)),
            last_tab: Default::default(),
        }
    }
}

impl<'a> Component for Layout<'a> {
    fn get_hotkeys(&self, state: &State) -> Vec<Hotkey> {
        if let Some(node_key) = state.nodeinfo {
            return self
                .nodeinfo_widget_state
                .get_hotkeys(state.my_node_key == Some(node_key));
        }

        match state.active_tab {
            Tab::Chat => self.chat_component.get_hotkeys(state),
            Tab::Nodes => self.nodes_component.get_hotkeys(state),
            Tab::Settings => self.settings_component.get_hotkeys(state),
            Tab::Connection => self.connection_component.get_hotkeys(state),
            Tab::Logs => self.logs_component.get_hotkeys(state),
        }
    }

    fn handle_event(
        &mut self,
        state: &State,
        event: &Event,
        emit: &impl Fn(AppEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<bool> {
        if let Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers,
            ..
        }) = event
            && modifiers.contains(KeyModifiers::CONTROL)
        {
            emit(AppEvent::QuitRequested)?;
            return Ok(true);
        }

        if let Some(node_key) = state.nodeinfo {
            if self.nodeinfo_widget_state.handle_event(
                build_nodeinfo_context(node_key, state),
                event.clone(),
                &mut |ev| match ev {
                    NodeInfoWidgetEvent::CloseRequested => {
                        emit(AppEvent::NodeInfoPopupCloseRequested)?;
                        Ok(())
                    }
                    NodeInfoWidgetEvent::CopyToClipboardRequested(s) => {
                        emit(AppEvent::CopyToClipboardRequested(s))?;
                        Ok(())
                    }
                    NodeInfoWidgetEvent::NodeDeleteRequested => {
                        emit(AppEvent::NodeDeleteRequested(node_key))?;
                        Ok(())
                    }
                    NodeInfoWidgetEvent::TracerouteRequested => {
                        emit(AppEvent::TracerouteRequested(node_key))?;
                        Ok(())
                    }
                },
            )? {
                return Ok(true);
            }

            return Ok(false);
        }

        let is_handled = match state.active_tab {
            Tab::Chat => self.chat_component.handle_event(state, event, emit),
            Tab::Nodes => self.nodes_component.handle_event(state, event, emit),
            Tab::Settings => self.settings_component.handle_event(state, event, emit),
            Tab::Connection => self.connection_component.handle_event(state, event, emit),
            Tab::Logs => self.logs_component.handle_event(state, event, emit),
        }?;

        if is_handled {
            return Ok(true);
        }

        if self.header_component.handle_event(state, event, emit)? {
            return Ok(true);
        }

        match event {
            Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                modifiers,
                ..
            }) => match code {
                KeyCode::Tab if modifiers.is_empty() => {
                    emit(AppEvent::TabNextRequested)?;
                    return Ok(true);
                }
                KeyCode::BackTab => {
                    emit(AppEvent::TabPreviousRequested)?;
                    return Ok(true);
                }
                KeyCode::F(12) if modifiers.is_empty() => {
                    emit(AppEvent::SplashLogoRequested)?;
                    return Ok(true);
                }
                KeyCode::Esc if modifiers.is_empty() => {
                    if self.last_esc_t.elapsed() < Duration::from_millis(300) {
                        emit(AppEvent::QuitRequested)?;
                    } else {
                        emit(AppEvent::TryingToQuit)?;
                    }

                    self.last_esc_t = Instant::now();
                }
                _ => {}
            },
            _ => {}
        }

        Ok(false)
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        if area.width < MIN_TERMINAL_SIZE.0 || area.height < MIN_TERMINAL_SIZE.1 {
            TerminalSizeWidget::new(MIN_TERMINAL_SIZE).render(area, frame.buffer_mut());
            return;
        }

        let container = Block::default().padding(Padding::new(
            if state.ui_config.is_left_padding_hidden { 0 } else { 2 },
            if state.ui_config.is_right_padding_hidden { 0 } else { 2 },
            if state.ui_config.is_top_padding_hidden { 0 } else { 1 },
            if state.ui_config.is_bottom_padding_hidden { 0 } else { 1 },
        ));

        let area = container.inner(frame.area());

        let v = RatatuiLayout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(area);

        self.header_component.render(state, frame, v[0]);

        TabsWidget::new(
            Tab::iter().map(|t| (t as usize, t.to_string())).collect(),
            state.active_tab as usize,
        )
        .render(v[2], frame.buffer_mut());

        if self.last_tab != state.active_tab {
            Clear.render(v[4], frame.buffer_mut());
            self.last_tab = state.active_tab;
        }

        match state.active_tab {
            Tab::Chat => self.chat_component.render(state, frame, v[4]),
            Tab::Nodes => self.nodes_component.render(state, frame, v[4]),
            Tab::Settings => self.settings_component.render(state, frame, v[4]),
            Tab::Connection => self.connection_component.render(state, frame, v[4]),
            Tab::Logs => self.logs_component.render(state, frame, v[4]),
        }

        // hotkeys
        HotkeysWidget::new(&self.get_hotkeys(state)).render(v[5], frame.buffer_mut());

        // node info popup
        if let Some(node_key) = state.nodeinfo {
            let popup_area = area.centered(Constraint::Length(60), Constraint::Length(17));

            Clear.render(popup_area, frame.buffer_mut());

            NodeInfoWidget::new(build_nodeinfo_context(node_key, state)).render(
                popup_area,
                frame.buffer_mut(),
                &mut self.nodeinfo_widget_state,
            );
        }

        // splash logo
        if state.splash_logo {
            let logo_popup_area = area.centered(
                Constraint::Length(self.logo.width() as u16),
                Constraint::Length(self.logo.height() as u16),
            );

            (&self.logo).render(logo_popup_area, frame.buffer_mut());
        }

        // toast
        if let Some(Toast { kind, text, .. }) = &state.toast {
            let toast_width = text.len() as u16 + 4;

            let toast_area = Rect {
                x: area.x + area.width / 2 - toast_width / 2,
                y: area.y + area.height - area.height / 6,
                width: toast_width,
                height: 3,
            };

            let (border_color, text_color) = match kind {
                ToastKind::Success => (Color::Green, Color::Green),
                ToastKind::Normal => (Color::DarkGray, Color::White),
                ToastKind::Warning => (Color::DarkGray, Color::Yellow),
                ToastKind::Error => (Color::Red, Color::Red),
            };

            let toast_block = Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .border_style(Style::new().fg(border_color))
                .padding(Padding::symmetric(1, 0));

            let toast_block_area = toast_block.inner(toast_area);

            Clear.render(toast_area, frame.buffer_mut());
            toast_block.render(toast_area, frame.buffer_mut());

            Paragraph::new(Span::from(text))
                .fg(text_color)
                .centered()
                .render(toast_block_area, frame.buffer_mut());
        }
    }
}

fn build_nodeinfo_context(node_key: u32, state: &State) -> NodeInfoContext<'_> {
    NodeInfoContext {
        node_key,
        nodes: &state.nodes,
        traceroutes: state
            .nodes_traceroutes
            .get(&node_key)
            .map(|ids| ids.iter().filter_map(|id| state.traceroutes.get(id)).sorted().collect())
            .unwrap_or_default(),
        telemetry: &state.nodeinfo_telemetry,
        uptime: state
            .nodes_last_telemetry
            .get(&node_key)
            .and_then(|t| t.device_metrics.as_ref())
            .and_then(|m| m.data.uptime_seconds),
        is_my_node: state.my_node_key == Some(node_key),
    }
}
