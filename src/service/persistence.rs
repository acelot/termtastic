use itertools::Itertools;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::repository::{Repository, create_sqlite_repository};
use crate::state::State;
use crate::types::{Node, Traceroute};
use crate::{
    APP_NAME,
    state::StateAction,
    types::{AppEvent, Toast},
};

pub struct PersistenceService {
    app_event_tx: broadcast::Sender<AppEvent>,
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    stored_action_rx: broadcast::Receiver<StateAction>,
    data_dir: PathBuf,
    repository: Option<Arc<Repository>>,
}

impl PersistenceService {
    pub fn new(
        app_event_tx: broadcast::Sender<AppEvent>,
        app_event_rx: broadcast::Receiver<AppEvent>,
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        stored_action_rx: broadcast::Receiver<StateAction>,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            app_event_tx,
            app_event_rx,
            state_rx,
            state_action_tx,
            stored_action_rx,
            data_dir,
            repository: None,
        }
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                event = self.stored_action_rx.recv() => self.handle_stored_action(event).await?,
                event = self.app_event_rx.recv() => self.handle_app_event(event).await?,
                _ = subsys.on_shutdown_requested() => {
                    self.repository.take();
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_stored_action(
        &mut self,
        event: Result<StateAction, broadcast::error::RecvError>,
    ) -> anyhow::Result<()> {
        let action = match event {
            Ok(action) => action,
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("broadcast receiver lagged by {} events", n);
                return Ok(());
            }
            Err(broadcast::error::RecvError::Closed) => return Ok(()),
        };

        let Some(repository) = &self.repository else {
            return Ok(());
        };

        match action {
            StateAction::NodeInit(Node { key, .. })
            | StateAction::NodeUpdate(Node { key, .. })
            | StateAction::NodeUpdateLastHeard { node_key: key, .. } => {
                let Some(node) = self
                    .state_rx
                    .borrow()
                    .nodes
                    .get(&key)
                    .filter(|node| node.user.is_some())
                    .cloned()
                else {
                    return Ok(());
                };

                if let Err(e) = repository.node_upsert(node.into()).await {
                    tracing::error!("node upsert failed: {}", e);

                    self.state_action_tx
                        .send(StateAction::Toast(Toast::error("DB error: see logs")))?;
                }
            }
            StateAction::NodeDelete(node_key) => {
                if let Err(e) = repository.node_remove(node_key).await {
                    tracing::error!("node delete failed: {}", e);

                    self.state_action_tx
                        .send(StateAction::Toast(Toast::error("DB error: see logs")))?;
                }
            }
            StateAction::TracerouteFinish { message_id, .. }
            | StateAction::TracerouteTimeout { message_id }
            | StateAction::MessageErrorSet { message_id, .. } => {
                let Some(traceroute) = self.state_rx.borrow().traceroutes.get(&message_id).cloned() else {
                    return Ok(());
                };

                let traceroute_entity = match traceroute.try_into() {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!("can't convert Traceroute into DB entity: {}", e);
                        return Ok(());
                    }
                };

                if let Err(e) = repository.traceroute_upsert(traceroute_entity).await {
                    tracing::error!("traceroute save failed: {}", e);

                    self.state_action_tx
                        .send(StateAction::Toast(Toast::error("DB error: see logs")))?;
                }
            }
            _ => {}
        };

        Ok(())
    }

    async fn handle_app_event(&mut self, event: Result<AppEvent, broadcast::error::RecvError>) -> anyhow::Result<()> {
        match event {
            Ok(event) => match event {
                AppEvent::DbLoadRequested(node_key) => {
                    let db_file = self.data_dir.join(format!("{}_{}.sqlite3", APP_NAME, node_key));

                    match create_sqlite_repository(db_file).await {
                        Ok(repository) => {
                            self.repository = Some(Arc::new(repository));
                        }
                        Err(e) => {
                            tracing::error!("DB initialization failed: {}", e);

                            self.state_action_tx
                                .send(StateAction::Toast(Toast::error("DB initialization failed")))?;
                        }
                    }

                    if self.load_data().await.is_ok() {
                        self.state_action_tx
                            .send(StateAction::Toast(Toast::normal("DB data loaded")))?;
                    }

                    self.app_event_tx.send(AppEvent::DbLoadFinished)?;
                }
                AppEvent::TelemetryArrived(packet) => {
                    let Some(repository) = &self.repository else {
                        return Ok(());
                    };

                    match repository.telemetry_insert(packet.try_into()?).await {
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!("telemetry store to DB failed: {}", e);
                        }
                    };
                }
                _ => {}
            },
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("broadcast receiver lagged by {} events", n);
            }
            _ => {}
        }

        Ok(())
    }

    async fn load_data(&self) -> anyhow::Result<()> {
        let Some(repository) = &self.repository else {
            tracing::warn!("DB data not loaded");

            self.state_action_tx
                .send(StateAction::Toast(Toast::warning("DB data not loaded")))?;

            return Ok(());
        };

        // nodes
        let nodes: Vec<Node> = match repository.node_find_all().await {
            Ok(nodes) => {
                tracing::info!("nodes loaded from DB: {}", nodes.len());

                nodes.into_iter().map_into().collect()
            }
            Err(e) => {
                tracing::error!("nodes load failed: {}", e);

                self.state_action_tx
                    .send(StateAction::Toast(Toast::error("nodes not loaded from DB")))?;

                return Ok(());
            }
        };

        self.state_action_tx.send(StateAction::DbNodesLoad(nodes))?;

        // traceroutes
        let traceroutes: Vec<Traceroute> = match repository.traceroute_find_all().await {
            Ok(entities) => {
                tracing::info!("traceroutes loaded from DB: {}", entities.len());

                entities
                    .into_iter()
                    .filter_map(|e| match Traceroute::try_from(e) {
                        Ok(t) => Some(t),
                        Err(e) => {
                            tracing::error!("can't convert DB entity to Traceroute: {}", e);
                            None
                        }
                    })
                    .collect()
            }
            Err(e) => {
                tracing::error!("traceroutes load failed: {}", e);

                self.state_action_tx
                    .send(StateAction::Toast(Toast::error("traceroutes not loaded from DB")))?;

                return Ok(());
            }
        };

        self.state_action_tx.send(StateAction::DbTracerouteLoad(traceroutes))?;

        // telemetry
        match repository.telemetry_find_last_for_each_node().await {
            Ok(map) => {
                for (_, telemetry) in map.iter() {
                    for item in telemetry {
                        match item.try_into() {
                            Ok(node_telemetry) => self
                                .state_action_tx
                                .send(StateAction::NodeLastTelemetrySet(node_telemetry))?,
                            Err(e) => {
                                tracing::error!(
                                    "telemetry conversion failed, id: {:?}, node_key: {}, kind: {}, error: {}",
                                    item.id,
                                    item.node_key,
                                    item.kind,
                                    e
                                )
                            }
                        }
                    }
                }

                tracing::info!(
                    "telemetry data loaded from DB: {} items for {} nodes",
                    map.values().map(|items| items.len()).sum::<usize>(),
                    map.len()
                );
            }
            Err(e) => {
                tracing::error!("telemetry load failed: {}", e);

                self.state_action_tx
                    .send(StateAction::Toast(Toast::error("telemetry not loaded from DB")))?;

                return Ok(());
            }
        };

        Ok(())
    }
}
