use itertools::Itertools;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_graceful_shutdown::SubsystemHandle;

use crate::repository::{Repository, create_sqlite_repository};
use crate::state::State;
use crate::types::{DbLoadState, Message, Node, NodePosition, NodeTelemetry, Traceroute};
use crate::{APP_NAME, state::StateAction, types::Toast};

const DATA_LOAD_TIMEOUT_SECS: u64 = 5;
const RECENT_MESSAGES_PER_CHAT_LIMIT: u32 = 100;
const NODEINFO_POSITION_LOAD_TIMEOUT_SECS: u64 = 2;
const NODEINFO_POSITION_LOAD_LIMIT: u32 = 100;

pub struct PersistenceService {
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    stored_action_rx: broadcast::Receiver<StateAction>,
    data_dir: PathBuf,
    db_tx: mpsc::Sender<()>,
    db_rx: mpsc::Receiver<()>,
    repository: Option<Arc<Repository>>,
    data_load_handle: Option<JoinHandle<()>>,
    nodeinfo_position_load_handle: Option<JoinHandle<()>>,
}

impl PersistenceService {
    pub fn new(
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        stored_action_rx: broadcast::Receiver<StateAction>,
        data_dir: PathBuf,
    ) -> Self {
        let (db_tx, db_rx) = mpsc::channel::<()>(1);

        Self {
            state_rx,
            state_action_tx,
            stored_action_rx,
            data_dir,
            db_tx,
            db_rx,
            repository: None,
            data_load_handle: None,
            nodeinfo_position_load_handle: None,
        }
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                event = self.stored_action_rx.recv() => self.handle_stored_action(event).await,
                Some(_) = self.db_rx.recv() => self.trigger_db_indicator(),
                _ = subsys.on_shutdown_requested() => {
                    self.repository = None;
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_stored_action(&mut self, event: Result<StateAction, broadcast::error::RecvError>) {
        let action = match event {
            Ok(action) => action,
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("broadcast receiver lagged by {} events", n);
                return;
            }
            Err(broadcast::error::RecvError::Closed) => return,
        };

        let is_db_loaded = matches!(self.state_rx.borrow().db_load_state, DbLoadState::Loaded);

        match action {
            StateAction::MyNodeInfoSet { node_key, .. } => {
                let _ = self.state_action_tx.send(StateAction::ConnectionLoadDbData);

                let db_file = self.data_dir.join(format!("{}_{}.sqlite3", APP_NAME, node_key));
                tracing::info!("DB file: {}", db_file.display());

                let repository = match create_sqlite_repository(db_file, node_key, self.db_tx.clone()).await {
                    Ok(repository) => {
                        tracing::info!("DB initialized successfully");

                        Arc::new(repository)
                    }
                    Err(e) => {
                        tracing::error!("DB initialization failed: {}", e);

                        let _ = self.state_action_tx.send(StateAction::DbLoadFail);

                        let _ = self
                            .state_action_tx
                            .send(StateAction::Toast(Toast::error("DB initialization failed")));

                        return;
                    }
                };

                let state_action_tx = self.state_action_tx.clone();
                let repository_clone = repository.clone();

                self.repository = Some(repository);

                self.data_load_handle = Some(tokio::spawn(async move {
                    match timeout(
                        Duration::from_secs(DATA_LOAD_TIMEOUT_SECS),
                        load_data(repository_clone, &state_action_tx),
                    )
                    .await
                    {
                        Ok(Ok(())) => {
                            tracing::info!("DB loading completed");

                            let _ = state_action_tx.send(StateAction::DbLoadSuccess);
                        }
                        Ok(Err(e)) => {
                            tracing::error!("DB loading failed: {}", e);

                            let _ = state_action_tx.send(StateAction::DbLoadFail);

                            let _ = state_action_tx.send(StateAction::Toast(Toast::error(
                                "An error occurred while loading DB data",
                            )));
                        }
                        Err(e) => {
                            tracing::error!("DB loading timed out: {}", e);

                            let _ = state_action_tx.send(StateAction::DbLoadFail);

                            let _ = state_action_tx.send(StateAction::Toast(Toast::error(
                                "An error occurred while loading DB data",
                            )));
                        }
                    };
                }));
            }
            StateAction::NodeInit(Node { key, .. })
            | StateAction::NodeUpdate(Node { key, .. })
            | StateAction::NodeUpdateLastHeard { node_key: key, .. }
                if !is_db_loaded =>
            {
                let Some(repository) = &self.repository else {
                    return;
                };

                if self.state_rx.borrow().is_my_node(key) {
                    return;
                }

                let Some(node_entity) = self.state_rx.borrow().nodes.get(&key).map(|n| n.into()) else {
                    return;
                };

                if let Err(e) = repository.node_upsert(node_entity).await {
                    tracing::error!("node upsert failed: {}", e);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::error("DB error: see logs")));
                }
            }
            StateAction::NodeDelete(node_key) if !is_db_loaded => {
                let Some(repository) = &self.repository else {
                    return;
                };

                if let Err(e) = repository.node_remove(node_key).await {
                    tracing::error!("node delete failed: {}", e);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::error("DB error: see logs")));
                }
            }
            StateAction::NodeLastTelemetrySet(node_telemetry) if !is_db_loaded => {
                let Some(repository) = &self.repository else {
                    return;
                };

                let telemetry_entity = match node_telemetry.try_into() {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!("can't convert NodeTelemetry into DB entity: {}", e);
                        return;
                    }
                };

                match repository.telemetry_insert(telemetry_entity).await {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("telemetry insert failed: {}", e);
                    }
                };
            }
            StateAction::NodeLastPositionSet(node_position) if !is_db_loaded => {
                let Some(repository) = &self.repository else {
                    return;
                };

                let position_entity = match node_position.try_into() {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!("can't convert NodePosition into DB entity: {}", e);
                        return;
                    }
                };

                match repository.position_insert(position_entity).await {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("position insert failed: {}", e);
                    }
                };
            }
            StateAction::NodeInfoKeySet(node_key) if !is_db_loaded => {
                let Some(repository) = &self.repository else {
                    return;
                };

                let state_action_tx = self.state_action_tx.clone();
                let repository_clone = repository.clone();

                self.nodeinfo_position_load_handle = Some(tokio::spawn(async move {
                    match timeout(
                        Duration::from_secs(NODEINFO_POSITION_LOAD_TIMEOUT_SECS),
                        repository_clone.position_find_by_node_key(node_key, NODEINFO_POSITION_LOAD_LIMIT),
                    )
                    .await
                    {
                        Ok(Ok(items)) => {
                            let _ = state_action_tx
                                .send(StateAction::NodeInfoPositionSet(items.iter().map(Into::into).collect()));
                        }
                        Ok(Err(e)) => {
                            tracing::error!("position loading failed: {}", e);

                            let _ = state_action_tx.send(StateAction::Toast(Toast::error(
                                "An error occurred while loading position data",
                            )));
                        }
                        Err(e) => {
                            tracing::error!("position loading timed out: {}", e);

                            let _ = state_action_tx.send(StateAction::Toast(Toast::error(
                                "An error occurred while loading position data",
                            )));
                        }
                    };
                }));
            }
            StateAction::NodeInfoKeyUnset => {
                if let Some(handle) = self.nodeinfo_position_load_handle.take() {
                    handle.abort();
                }
            }
            StateAction::TracerouteFinish { message_id, .. } | StateAction::TracerouteTimeout { message_id }
                if !is_db_loaded =>
            {
                let Some(repository) = &self.repository else {
                    return;
                };

                let Some(traceroute) = self.state_rx.borrow().traceroutes.get(&message_id).cloned() else {
                    return;
                };

                let traceroute_entity = match traceroute.try_into() {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!("can't convert Traceroute into DB entity: {}", e);
                        return;
                    }
                };

                if let Err(e) = repository.traceroute_upsert(traceroute_entity).await {
                    tracing::error!("traceroute upsert failed: {}", e);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::error("DB error: see logs")));
                }
            }
            StateAction::MessageAdd(message) if !is_db_loaded => {
                let Some(repository) = &self.repository else {
                    return;
                };

                if let Err(e) = repository.message_upsert(message.into()).await {
                    tracing::error!("message upsert failed: {}", e);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::error("DB error: see logs")));
                }
            }
            StateAction::MessageErrorSet { message_id, error, .. } if !is_db_loaded => {
                let Some(repository) = &self.repository else {
                    return;
                };

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
            StateAction::ChatPurge(chat) if !is_db_loaded => {
                let Some(repository) = &self.repository else {
                    return;
                };

                let my_node_key = self.state_rx.borrow().my_node_key.expect("should be Some");

                if let Err(e) = repository.message_remove_by_chat(chat, my_node_key).await {
                    tracing::error!("chat purge failed: {}", e);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::error("DB error: see logs")));
                }
            }
            StateAction::DbLoadFail | StateAction::ConnectionStop => {
                if let Some(handle) = self.nodeinfo_position_load_handle.take() {
                    handle.abort();
                }

                if let Some(handle) = self.data_load_handle.take() {
                    handle.abort();
                }

                self.repository = None;

                tracing::debug!("DB closed");
            }
            _ => {}
        };
    }

    fn trigger_db_indicator(&mut self) {
        let _ = self.state_action_tx.send(StateAction::DbIndicatorTrigger);
    }
}

async fn load_data(repository: Arc<Repository>, state_action_tx: &UnboundedSender<StateAction>) -> anyhow::Result<()> {
    // nodes
    let nodes: Vec<Node> = repository.node_find_all().await?.into_iter().map_into().collect();

    tracing::info!("nodes loaded from DB: {}", nodes.len());

    let _ = state_action_tx.send(StateAction::DbNodesLoad(nodes));

    // messages
    let messages: Vec<Message> = repository
        .message_find_recent_for_each_chat(RECENT_MESSAGES_PER_CHAT_LIMIT)
        .await?
        .into_iter()
        .map_into()
        .collect();

    tracing::info!("messages loaded from DB: {}", messages.len());

    let _ = state_action_tx.send(StateAction::DbMessagesLoad(messages));

    // traceroutes
    let traceroutes: Vec<Traceroute> = repository
        .traceroute_find_all()
        .await?
        .into_iter()
        .filter_map(|entity| match Traceroute::try_from(&entity) {
            Ok(t) => Some(t),
            Err(e) => {
                tracing::warn!(
                    "can't convert DB entity to Traceroute, id: {}, error: {}",
                    entity.message_id,
                    e
                );
                None
            }
        })
        .collect();

    tracing::info!("traceroutes loaded from DB: {}", traceroutes.len());

    let _ = state_action_tx.send(StateAction::DbTracerouteLoad(traceroutes));

    // telemetry
    let last_telemetry: Vec<NodeTelemetry> = repository
        .telemetry_find_last_for_each_node()
        .await?
        .iter()
        .map(|(_, telemetry)| {
            telemetry
                .iter()
                .filter_map(|entity| match NodeTelemetry::try_from(entity) {
                    Ok(t) => Some(t),
                    Err(e) => {
                        tracing::error!(
                            "can't convert DB entity to NodeTelemetry, id: {:?}, node_key: {}, kind: {}, error: {}",
                            entity.id,
                            entity.node_key,
                            entity.kind,
                            e
                        );
                        None
                    }
                })
                .collect::<Vec<NodeTelemetry>>()
        })
        .flatten()
        .collect();

    tracing::info!("telemetry data loaded from DB: {}", last_telemetry.len());

    let _ = state_action_tx.send(StateAction::DbNodesLastTelemetryLoad(last_telemetry));

    // position
    let last_position: HashMap<u32, NodePosition> = repository
        .position_find_last_for_each_node()
        .await?
        .iter()
        .map(|(node_key, position_entity)| (*node_key, position_entity.into()))
        .collect();

    tracing::info!("position data loaded from DB: {}", last_position.len());

    let _ = state_action_tx.send(StateAction::DbNodesLastPositionLoad(last_position));

    Ok(())
}
