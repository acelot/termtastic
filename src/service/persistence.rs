use itertools::Itertools;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::repository::{Repository, create_sqlite_repository};
use crate::state::State;
use crate::types::{Message, Node, NodePosition, NodeTelemetry, Traceroute};
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
    db_tx: mpsc::Sender<()>,
    db_rx: mpsc::Receiver<()>,
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
        let (db_tx, db_rx) = mpsc::channel::<()>(1);

        Self {
            app_event_tx,
            app_event_rx,
            state_rx,
            state_action_tx,
            stored_action_rx,
            data_dir,
            repository: None,
            db_tx,
            db_rx,
        }
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                event = self.stored_action_rx.recv() => self.handle_stored_action(event).await?,
                event = self.app_event_rx.recv() => self.handle_app_event(event).await?,
                event = self.db_rx.recv() => self.handle_db_event(event)?,
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
                if self.state_rx.borrow().is_my_node(key) {
                    return Ok(());
                }

                let Some(node_entity) = self.state_rx.borrow().nodes.get(&key).map(|n| n.into()) else {
                    return Ok(());
                };

                if let Err(e) = repository.node_upsert(node_entity).await {
                    tracing::error!("node upsert failed: {}", e);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::error("DB error: see logs")));
                }
            }
            StateAction::NodeDelete(node_key) => {
                if let Err(e) = repository.node_remove(node_key).await {
                    tracing::error!("node delete failed: {}", e);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::error("DB error: see logs")));
                }
            }
            StateAction::NodeLastTelemetrySet(node_telemetry) => {
                let telemetry_entity = match node_telemetry.try_into() {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!("can't convert NodeTelemetry into DB entity: {}", e);
                        return Ok(());
                    }
                };

                match repository.telemetry_insert(telemetry_entity).await {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("telemetry insert failed: {}", e);
                    }
                };
            }
            StateAction::NodeLastPositionSet(node_position) => {
                let position_entity = match node_position.try_into() {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!("can't convert NodePosition into DB entity: {}", e);
                        return Ok(());
                    }
                };

                match repository.position_insert(position_entity).await {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("position insert failed: {}", e);
                    }
                };
            }
            StateAction::NodeInfoKeySet(node_key) => match repository.position_find_by_node_key(node_key, 100).await {
                Ok(items) => {
                    let _ = self
                        .state_action_tx
                        .send(StateAction::NodeInfoPositionSet(items.iter().map(Into::into).collect()));
                }
                Err(e) => {
                    tracing::error!("position loading failed: {}", e);

                    let _ = self.state_action_tx.send(StateAction::Toast(Toast::error(
                        "An error occurred while loading position data",
                    )));
                }
            },
            StateAction::TracerouteFinish { message_id, .. } | StateAction::TracerouteTimeout { message_id } => {
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
                    tracing::error!("traceroute upsert failed: {}", e);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::error("DB error: see logs")));
                }
            }
            StateAction::MessageAdd(message) => {
                if let Err(e) = repository.message_upsert(message.into()).await {
                    tracing::error!("message upsert failed: {}", e);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::error("DB error: see logs")));
                }
            }
            StateAction::MessageErrorSet { message_id, error, .. } => {
                if let Err(e) = repository
                    .message_update_routing_error(message_id, error.map(|e| e as i32))
                    .await
                {
                    tracing::error!("message update failed: {}", e);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::error("DB error: see logs")));
                }
            }
            StateAction::ChatPurge(chat) => {
                let my_node_key = self.state_rx.borrow().my_node_key.expect("should be Some");

                if let Err(e) = repository.message_remove_by_chat(chat, my_node_key).await {
                    tracing::error!("chat purge failed: {}", e);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::error("DB error: see logs")));
                }
            }
            StateAction::ConnectionStop => {
                self.repository.take();
                tracing::debug!("DB closed");
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
                    tracing::debug!("DB file: {}", db_file.display());

                    let repository = match create_sqlite_repository(db_file, self.db_tx.clone()).await {
                        Ok(repository) => repository,
                        Err(e) => {
                            tracing::error!("DB initialization failed: {}", e);

                            let _ = self
                                .state_action_tx
                                .send(StateAction::Toast(Toast::error("DB initialization failed")));

                            return Ok(());
                        }
                    };

                    self.load_data(&repository).await;
                    self.repository = Some(Arc::new(repository));
                    let _ = self.app_event_tx.send(AppEvent::DbLoadFinished);
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

    fn handle_db_event(&mut self, maybe_event: Option<()>) -> anyhow::Result<()> {
        if maybe_event.is_some() {
            let _ = self.state_action_tx.send(StateAction::DbIndicatorTrigger);
        }

        Ok(())
    }

    async fn load_data(&self, repository: &Repository) {
        // nodes
        let nodes: Vec<Node> = match repository.node_find_all().await {
            Ok(nodes) => {
                tracing::info!("nodes loaded from DB: {}", nodes.len());

                nodes.into_iter().map_into().collect()
            }
            Err(e) => {
                tracing::error!("nodes load failed: {}", e);

                let _ = self
                    .state_action_tx
                    .send(StateAction::Toast(Toast::error("nodes not loaded from DB")));

                return;
            }
        };

        let _ = self.state_action_tx.send(StateAction::DbNodesLoad(nodes));

        // messages
        let messages: Vec<Message> = match repository.message_find_all().await {
            Ok(messages) => {
                tracing::info!("messages loaded from DB: {}", messages.len());

                messages.into_iter().map_into().collect()
            }
            Err(e) => {
                tracing::error!("messages load failed: {}", e);

                let _ = self
                    .state_action_tx
                    .send(StateAction::Toast(Toast::error("messages not loaded from DB")));

                return;
            }
        };

        let _ = self.state_action_tx.send(StateAction::DbMessagesLoad(messages));

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

                let _ = self
                    .state_action_tx
                    .send(StateAction::Toast(Toast::error("traceroutes not loaded from DB")));

                return;
            }
        };

        let _ = self.state_action_tx.send(StateAction::DbTracerouteLoad(traceroutes));

        // telemetry
        match repository.telemetry_find_last_for_each_node().await {
            Ok(map) => {
                let last_telemetry: Vec<NodeTelemetry> = map
                    .iter()
                    .map(|(_, telemetry)| {
                        telemetry
                            .iter()
                            .filter_map(|item| match NodeTelemetry::try_from(item) {
                                Ok(t) => Some(t),
                                Err(e) => {
                                    tracing::error!(
                                        "can't convert DB entity to NodeTelemetry, id: {:?}, node_key: {}, kind: {}, error: {}",
                                        item.id,
                                        item.node_key,
                                        item.kind,
                                        e
                                    );
                                    None
                                }
                            })
                            .collect::<Vec<NodeTelemetry>>()
                    })
                    .flatten()
                    .collect();

                if !last_telemetry.is_empty() {
                    tracing::info!("telemetry data loaded from DB: {}", last_telemetry.len());

                    let _ = self
                        .state_action_tx
                        .send(StateAction::DbNodesLastTelemetryLoad(last_telemetry));
                }
            }
            Err(e) => {
                tracing::error!("telemetry load failed: {}", e);

                let _ = self
                    .state_action_tx
                    .send(StateAction::Toast(Toast::error("telemetry not loaded from DB")));

                return;
            }
        };

        // position
        match repository.position_find_last_for_each_node().await {
            Ok(map) => {
                let last_position: HashMap<u32, NodePosition> = map
                    .iter()
                    .map(|(node_key, position_entity)| (*node_key, position_entity.into()))
                    .collect();

                tracing::info!("position data loaded from DB: {}", last_position.len());

                let _ = self
                    .state_action_tx
                    .send(StateAction::DbNodesLastPositionLoad(last_position));
            }
            Err(e) => {
                tracing::error!("position load failed: {}", e);

                let _ = self
                    .state_action_tx
                    .send(StateAction::Toast(Toast::error("position data not loaded from DB")));

                return;
            }
        };

        let _ = self
            .state_action_tx
            .send(StateAction::Toast(Toast::normal("DB data loaded")));
    }
}
