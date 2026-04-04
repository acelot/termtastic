use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::ui::prelude::AppEvent;
use crate::{
    meshtastic::types::{CommandToMeshtastic, MeshtasticEvent},
    state::{State, StateAction},
};

#[allow(dead_code)]
pub struct UiService {
    app_event_tx: broadcast::Sender<AppEvent>,
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
}

impl UiService {
    pub fn new(
        app_event_tx: broadcast::Sender<AppEvent>,
        app_event_rx: broadcast::Receiver<AppEvent>,
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
        meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
    ) -> Self {
        Self {
            app_event_tx,
            app_event_rx,
            state_rx,
            state_action_tx,
            meshtastic_command_tx,
            meshtastic_event_rx,
        }
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                Ok(event) = self.app_event_rx.recv() => self.handle_app_event(event)?,
                Ok(event) = self.meshtastic_event_rx.recv() => self.handle_meshtastic_event(event)?,
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_app_event(&mut self, event: AppEvent) -> anyhow::Result<()> {
        match event {
            AppEvent::InitializationRequested | AppEvent::SplashLogoRequested => {
                self.state_action_tx.send(StateAction::SplashLogo)?;
            }
            AppEvent::NextTabRequested => {
                self.state_action_tx.send(StateAction::TabSwitchToNext)?;
            }
            AppEvent::PreviousTabRequested => {
                self.state_action_tx
                    .send(StateAction::TabSwitchToPrevious)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_meshtastic_event(&self, event: MeshtasticEvent) -> anyhow::Result<()> {
        match event {
            _ => {}
        }

        Ok(())
    }
}
