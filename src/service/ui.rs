use arboard::Clipboard;
use tokio::sync::{broadcast, mpsc};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::state::StateAction;
use crate::types::Toast;
use crate::ui::prelude::AppEvent;

pub struct UiService {
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
}

impl UiService {
    pub fn new(
        app_event_rx: broadcast::Receiver<AppEvent>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
    ) -> Self {
        Self {
            app_event_rx,
            state_action_tx,
        }
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                event = self.app_event_rx.recv() => self.handle_app_event(event, subsys),
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_app_event(&mut self, event: Result<AppEvent, broadcast::error::RecvError>, subsys: &mut SubsystemHandle) {
        match event {
            Ok(app_event) => match app_event {
                AppEvent::InitializationRequested | AppEvent::SplashLogoRequested => {
                    let _ = self.state_action_tx.send(StateAction::SplashLogo);
                }
                AppEvent::TabNextRequested => {
                    let _ = self.state_action_tx.send(StateAction::TabSwitchToNext);
                }
                AppEvent::TabPreviousRequested => {
                    let _ = self.state_action_tx.send(StateAction::TabSwitchToPrevious);
                }
                AppEvent::CopyToClipboardRequested(text) => match copy_to_clipboard(text) {
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
                AppEvent::TryingToQuit => {
                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::normal("press <Esc> twice quickly to quit")));
                }
                AppEvent::QuitRequested => {
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
