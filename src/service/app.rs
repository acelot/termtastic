use arboard::Clipboard;
use meshtastic::protobufs::config;
use tokio::sync::{broadcast, mpsc};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::state::{State, StateAction};
use crate::types::{ChannelRole, Chat, Tab, Toast};
use crate::ui::helpers::ModemPresetExt;
use crate::ui::prelude::UiEvent;

pub struct AppService {
    ui_event_rx: broadcast::Receiver<UiEvent>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    clipboard: Option<Clipboard>,
}

impl AppService {
    pub fn new(ui_event_rx: broadcast::Receiver<UiEvent>, state_action_tx: mpsc::UnboundedSender<StateAction>) -> Self {
        let clipboard = Clipboard::new();
        if let Err(err) = &clipboard {
            tracing::warn!("Failed to acquire clipboard: {err}");
        }
        Self {
            ui_event_rx,
            state_action_tx,
            clipboard: clipboard.ok(),
        }
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                event = self.ui_event_rx.recv() => self.handle_ui_event(event, subsys),
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_ui_event(&mut self, event: Result<UiEvent, broadcast::error::RecvError>, subsys: &mut SubsystemHandle) {
        match event {
            Ok(app_event) => match app_event {
                UiEvent::InitializationRequested => {
                    let _ = self.state_action_tx.send(StateAction::SplashLogo);
                }
                UiEvent::TabNextRequested => {
                    let _ = self.state_action_tx.send(StateAction::TabSwitchToNext);
                }
                UiEvent::TabPreviousRequested => {
                    let _ = self.state_action_tx.send(StateAction::TabSwitchToPrevious);
                }
                UiEvent::CopyToClipboardRequested(text) => {
                    if let Some(clipboard) = self.clipboard.as_mut() {
                        match clipboard.set_text(text) {
                            Ok(_) => {
                                let _ = self.state_action_tx.send(StateAction::Toast(Toast::normal("copied")));
                            }
                            Err(e) => {
                                let _ = self
                                    .state_action_tx
                                    .send(StateAction::Toast(Toast::error("copy failed")));

                                tracing::error!("copy failed: {:?}", e);
                            }
                        }
                    } else {
                        let _ = self
                            .state_action_tx
                            .send(StateAction::Toast(Toast::error("clipboard is not available")));

                        tracing::error!("copy failed: clipboard is not available");
                    }
                }
                UiEvent::TryingToQuit => {
                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::normal("press <Esc> twice quickly to quit")));
                }
                UiEvent::QuitRequested => {
                    subsys.request_shutdown();
                }
                _ => {}
            },
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("broadcast receiver lagged by {} events", n);
            }
            _ => {}
        }
    }
}

pub fn update_terminal_title(state: &mut State) {
    let title = match state.active_tab {
        Tab::Chat => match state.active_chat {
            Some(Chat::Channel(channel_key)) => {
                let radio_preset_name = state
                    .device_config
                    .lora
                    .as_ref()
                    .and_then(|lora| config::lo_ra_config::ModemPreset::try_from(lora.modem_preset).ok())
                    .map(|preset| preset.as_channel_name());

                match state.channels.get(&channel_key) {
                    Some(channel) => match channel.role {
                        ChannelRole::Primary => Some(format!(
                            "#{} {}",
                            &channel.key,
                            if !channel.name.is_empty() {
                                channel.name.clone()
                            } else if let Some(preset_name) = radio_preset_name {
                                preset_name.clone()
                            } else {
                                "Primary".to_owned()
                            }
                        )),
                        ChannelRole::Secondary => Some(format!(
                            "#{} {}",
                            channel.key,
                            if !channel.name.is_empty() {
                                &channel.name
                            } else {
                                "Secondary"
                            }
                        )),
                        ChannelRole::Disabled => Some(format!("#{} Secondary Disabled", channel.key)),
                    },
                    None => Some(format!("!{:x}", channel_key)),
                }
            }
            Some(Chat::Direct(node_key)) => {
                let node = state.nodes.get(&node_key);

                match node {
                    Some(node) => Some(format!("[{}]", node.short_name())),
                    None => Some(format!("!{:x}", node_key)),
                }
            }
            None => Some(Tab::Chat.to_string()),
        },
        tab => Some(tab.to_string()),
    };

    let node_name = state.get_my_node().map(|n| format!(" · {}", n.short_name()));

    state.terminal_title = Some(format!(
        "{}{}",
        title.unwrap_or_default(),
        node_name.unwrap_or_default()
    ));
}
