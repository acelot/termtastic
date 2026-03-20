use std::hash::{DefaultHasher, Hash, Hasher};

use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;
use tracing_unwrap::ResultExt;

use crate::{
    state::{State, StateAction},
    types::{AppConfig, AppEvent, DevicesConfig},
};

pub struct ConfigService {
    event_tx: broadcast::Sender<AppEvent>,
    event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    app_config_last_hash: u64,
    device_config_last_hash: u64,
}

impl ConfigService {
    pub fn new(
        event_tx: broadcast::Sender<AppEvent>,
        event_rx: broadcast::Receiver<AppEvent>,
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
    ) -> Self {
        Self {
            event_tx,
            event_rx,
            state_rx,
            state_action_tx,
            app_config_last_hash: 0,
            device_config_last_hash: 0,
        }
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                Ok(event) = self.event_rx.recv() => self.handle_app_event(event).await,
                _ = self.state_rx.changed() => self.handle_state_change(),
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::InitializationRequested => {
                let state = &self.state_rx.borrow();

                let app_config: AppConfig = confy::load(&state.app_name, "app").unwrap_or_log();

                self.state_action_tx
                    .send(StateAction::SetAppConfig(app_config))
                    .unwrap_or_log();

                let app_config_devices: DevicesConfig =
                    confy::load(&state.app_name, "devices").unwrap_or_log();

                self.device_config_last_hash = calculate_hash(&app_config_devices);

                self.state_action_tx
                    .send(StateAction::SetAppConfigDevices(app_config_devices))
                    .unwrap_or_log();
            }
            _ => {}
        }
    }

    fn handle_state_change(&mut self) {
        let state = &self.state_rx.borrow();

        let app_config_hash = calculate_hash(&state.app_config);

        if app_config_hash != self.app_config_last_hash {
            confy::store(&state.app_name, "app", &state.app_config).unwrap_or_log();

            self.app_config_last_hash = app_config_hash;
        }

        let device_config_hash = calculate_hash(&state.devices_config);

        if device_config_hash != self.device_config_last_hash {
            confy::store(&state.app_name, "devices", &state.devices_config).unwrap_or_log();

            self.device_config_last_hash = device_config_hash;
        }
    }
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
