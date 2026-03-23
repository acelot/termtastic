use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;
use tracing_unwrap::ResultExt;

use crate::ui::prelude::AppEvent;
use crate::{
    meshtastic::types::{MeshtasticCommand, MeshtasticEvent},
    state::{State, StateAction},
};

pub struct UiService {
    app_event_tx: broadcast::Sender<AppEvent>,
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    meshtastic_command_tx: mpsc::UnboundedSender<MeshtasticCommand>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
}

impl UiService {
    pub fn new(
        app_event_tx: broadcast::Sender<AppEvent>,
        app_event_rx: broadcast::Receiver<AppEvent>,
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        meshtastic_command_tx: mpsc::UnboundedSender<MeshtasticCommand>,
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
                Ok(event) = self.app_event_rx.recv() => self.handle_app_event(event),
                Ok(event) = self.meshtastic_event_rx.recv() => self.handle_meshtastic_event(event),
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_app_event(&self, event: AppEvent) {
        match event {
            AppEvent::NextTabRequested => {
                self.state_action_tx
                    .send(StateAction::NextTab)
                    .unwrap_or_log();
            }
            AppEvent::PreviousTabRequested => {
                self.state_action_tx
                    .send(StateAction::PrevTab)
                    .unwrap_or_log();
            }
            _ => {}
        }
    }

    fn handle_meshtastic_event(&self, event: MeshtasticEvent) {
        match event {
            _ => {}
        }
    }
}
