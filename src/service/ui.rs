use arboard::Clipboard;
use tokio::sync::{broadcast, mpsc};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::state::StateAction;
use crate::types::Toast;
use crate::ui::prelude::UiEvent;

pub struct UiService {
    ui_event_rx: broadcast::Receiver<UiEvent>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
}

impl UiService {
    pub fn new(ui_event_rx: broadcast::Receiver<UiEvent>, state_action_tx: mpsc::UnboundedSender<StateAction>) -> Self {
        Self {
            ui_event_rx,
            state_action_tx,
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
                UiEvent::InitializationRequested | UiEvent::SplashLogoRequested => {
                    let _ = self.state_action_tx.send(StateAction::SplashLogo);
                }
                UiEvent::TabNextRequested => {
                    let _ = self.state_action_tx.send(StateAction::TabSwitchToNext);
                }
                UiEvent::TabPreviousRequested => {
                    let _ = self.state_action_tx.send(StateAction::TabSwitchToPrevious);
                }
                UiEvent::CopyToClipboardRequested(text) => match copy_to_clipboard(text) {
                    Ok(_) => {
                        let _ = self.state_action_tx.send(StateAction::Toast(Toast::normal("copied")));
                    }
                    Err(e) => {
                        let _ = self
                            .state_action_tx
                            .send(StateAction::Toast(Toast::error("copy failed")));

                        tracing::error!("copy failed: {:?}", e);
                    }
                },
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

fn copy_to_clipboard(text: String) -> anyhow::Result<()> {
    Clipboard::new()?.set_text(text)?;
    Ok(())
}
