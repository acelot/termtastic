use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::state::State;
use crate::{
    state::StateAction,
    types::{AppConfig, AppEvent, Toast},
};

pub struct ConfigService {
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    stored_action_rx: broadcast::Receiver<StateAction>,
    config_path: PathBuf,
    app_config_last_hash: u64,
}

impl ConfigService {
    pub fn new(
        app_event_rx: broadcast::Receiver<AppEvent>,
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        stored_action_rx: broadcast::Receiver<StateAction>,
        config_dir: PathBuf,
    ) -> Self {
        Self {
            app_event_rx,
            state_rx,
            state_action_tx,
            stored_action_rx,
            config_path: config_dir.join("app.ron"),
            app_config_last_hash: 0,
        }
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                event = self.app_event_rx.recv() => self.handle_app_event(event)?,
                event = self.stored_action_rx.recv() => self.handle_stored_action(event)?,
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_app_event(&mut self, event: Result<AppEvent, broadcast::error::RecvError>) -> anyhow::Result<()> {
        match event {
            Ok(AppEvent::InitializationRequested) => match confy::load_path::<AppConfig>(self.config_path.as_path()) {
                Ok(app_config) => {
                    self.state_action_tx.send(StateAction::AppConfigApply(app_config))?;
                }
                Err(e) => {
                    tracing::error!("can't load app config: {}", e);

                    self.state_action_tx
                        .send(StateAction::Toast(Toast::error("app config load failed")))?;
                }
            },
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("broadcast receiver lagged by {} events", n);
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_stored_action(&mut self, event: Result<StateAction, broadcast::error::RecvError>) -> anyhow::Result<()> {
        match event {
            Ok(action) => {
                if matches!(action, StateAction::AppConfigApply(_)) {
                    self.state_action_tx
                        .send(StateAction::Toast(Toast::normal("config loaded")))?;
                }

                let state = self.state_rx.borrow();
                let app_config: AppConfig = (&*state).into();
                let app_config_hash = calculate_hash(&app_config);

                if app_config_hash != self.app_config_last_hash {
                    confy::store_path(self.config_path.as_path(), &app_config)?;
                    self.app_config_last_hash = app_config_hash;
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("broadcast receiver lagged by {} events", n);
            }
            _ => {}
        }

        Ok(())
    }
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
