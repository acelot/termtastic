use std::time::Duration;

use chrono::Utc;
use meshtastic::{
    Message as _,
    protobufs::{
        AdminMessage, MeshPacket, PortNum, Telemetry, User, admin_message, from_radio, log_record,
        mesh_packet::{self},
    },
};
use tokio::{
    sync::{broadcast, mpsc, watch},
    time,
};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::types::{NodeTelemetry, Toast};
use crate::{
    meshtastic::types::{CommandToMeshtastic, MeshtasticEvent},
    state::StateAction,
    types::{AppEvent, Node},
};
use crate::{state::State, ui::helpers::DateTimeExt};

pub const ONLINE_NODE_THRESHOLD_SECS: i64 = 7200;
const UPDATE_ONLINE_NODES_INTERVAL_SECS: u64 = 2;

pub struct NodesService {
    app_event_tx: broadcast::Sender<AppEvent>,
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
    local_my_node_num: Option<u32>,
}

impl NodesService {
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
            local_my_node_num: None,
        }
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        let mut online_nodes_interval = time::interval(Duration::from_secs(UPDATE_ONLINE_NODES_INTERVAL_SECS));

        loop {
            tokio::select! {
                event = self.app_event_rx.recv() => self.handle_app_event(event)?,
                event = self.meshtastic_event_rx.recv() => self.handle_meshtastic_event(event)?,
                _ = online_nodes_interval.tick() => self.update_online_nodes()?,
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_app_event(&self, event: Result<AppEvent, broadcast::error::RecvError>) -> anyhow::Result<()> {
        let state = &self.state_rx.borrow();

        match event {
            Ok(app_event) => match app_event {
                AppEvent::DbLoadFinished => {
                    for node in &state.nodes_stash {
                        self.state_action_tx.send(StateAction::NodeInit(node.clone()))?;
                    }

                    self.state_action_tx.send(StateAction::NodesStashFlush)?;
                }
                AppEvent::DirectChatRequested(node_key) => {
                    self.state_action_tx.send(StateAction::DirectChatStart(node_key))?;
                }
                AppEvent::NodesSortByPrevRequested => {
                    self.state_action_tx
                        .send(StateAction::NodesSortBySet(state.nodes_sort_by.prev()))?;
                }
                AppEvent::NodesSortByNextRequested => {
                    self.state_action_tx
                        .send(StateAction::NodesSortBySet(state.nodes_sort_by.next()))?;
                }
                AppEvent::NodesFilterChanged(filter) => {
                    self.state_action_tx.send(StateAction::NodesFilterSet(filter))?;
                }
                AppEvent::NodeInfoBroadcastRequested => {
                    let my_node = state.get_my_node().expect("should be Some");

                    self.meshtastic_command_tx
                        .send(CommandToMeshtastic::BroadcastNodeInfo {
                            channel_id: 0,
                            user: my_node.try_into()?,
                        })?;
                }
                AppEvent::NodeInfoPopupOpenRequested(node_key) => {
                    self.state_action_tx.send(StateAction::NodeInfoSet(node_key))?;
                }
                AppEvent::NodeInfoPopupCloseRequested => {
                    self.state_action_tx.send(StateAction::NodeInfoUnset)?;
                }
                AppEvent::NodeDeleteRequested(node_key) => {
                    let my_node_num = state.my_node_key.expect("should be Some");

                    self.meshtastic_command_tx.send(CommandToMeshtastic::DeleteNode {
                        node_num: node_key,
                        my_node_num,
                    })?;
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

    fn handle_meshtastic_event(
        &mut self,
        event: Result<MeshtasticEvent, broadcast::error::RecvError>,
    ) -> anyhow::Result<()> {
        match event {
            Ok(meshtastic_event) => match meshtastic_event {
                MeshtasticEvent::IncomingPacket(packet) => self.handle_meshtastic_packet(*packet)?,
                MeshtasticEvent::NodeInfoBroadcastSent => self
                    .state_action_tx
                    .send(StateAction::Toast(Toast::success("NodeInfo broadcast sent")))?,
                MeshtasticEvent::NodeInfoBroadcastFailed(e) => {
                    tracing::error!("NodeInfo broadcast failed: {:?}", e);

                    self.state_action_tx
                        .send(StateAction::Toast(Toast::error("NodeInfo broadcast failed")))?;
                }
                MeshtasticEvent::NodeRemoveAccepted => self
                    .state_action_tx
                    .send(StateAction::Toast(Toast::success("node removed")))?,
                MeshtasticEvent::NodeRemoveFailed(e) => {
                    tracing::error!("node remove failed: {:?}", e);

                    self.state_action_tx
                        .send(StateAction::Toast(Toast::error("node remove failed")))?;
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

    fn handle_meshtastic_packet(&mut self, packet: from_radio::PayloadVariant) -> anyhow::Result<()> {
        match packet {
            from_radio::PayloadVariant::MyInfo(my_info) => {
                self.local_my_node_num = Some(my_info.my_node_num);

                self.state_action_tx
                    .send(StateAction::MyNodeKeySet(my_info.my_node_num))?;
            }
            from_radio::PayloadVariant::NodeInfo(node_info) => {
                match Node::try_from(&node_info) {
                    Ok(node) => {
                        let node_key = node.key;
                        let last_heard = node.last_heard.unwrap_or(Utc::now());

                        self.state_action_tx.send(StateAction::NodesStashPush(node))?;

                        if let Some(device_metrics) = node_info.device_metrics {
                            self.state_action_tx
                                .send(StateAction::NodeLastTelemetrySet(NodeTelemetry {
                                    node_key,
                                    datetime: last_heard,
                                    variant: meshtastic::protobufs::telemetry::Variant::DeviceMetrics(device_metrics),
                                }))?;
                        }
                    }
                    Err(e) => {
                        tracing::debug!(node_key = node_info.num, "can't convert NodeInfo into Node: {}", e);
                    }
                };
            }
            from_radio::PayloadVariant::Packet(mesh_packet) => {
                self.state_action_tx
                    .send(StateAction::NodeInit((&mesh_packet).into()))?;

                match &mesh_packet.payload_variant {
                    Some(mesh_packet::PayloadVariant::Decoded(data)) => match data.portnum() {
                        PortNum::NodeinfoApp => match User::decode(&*data.payload) {
                            Ok(user) => {
                                match Node::try_from((&mesh_packet, &user)) {
                                    Ok(node) => self.state_action_tx.send(StateAction::NodeUpdate(node))?,
                                    Err(e) => {
                                        tracing::debug!(
                                            node_key = mesh_packet.from,
                                            "can't convert NodeInfo into Node: {:?}",
                                            e
                                        );
                                    }
                                };
                            }
                            Err(e) => {
                                tracing::debug!("can't decode NodeinfoApp payload: {:?}", e);
                            }
                        },
                        PortNum::AdminApp => match AdminMessage::decode(&*data.payload) {
                            Ok(admin_message) => match admin_message.payload_variant {
                                Some(admin_message::PayloadVariant::SetOwner(user)) => {
                                    self.state_action_tx.send(StateAction::NodeOwnerSet((&user).into()))?;
                                }
                                Some(admin_message::PayloadVariant::RemoveByNodenum(node_num)) => {
                                    self.state_action_tx.send(StateAction::NodeDelete(node_num))?;
                                }
                                _ => {}
                            },
                            Err(e) => {
                                tracing::debug!("can't decode AdminMessage payload: {:?}", e);
                            }
                        },
                        PortNum::TelemetryApp => match Telemetry::decode(&*data.payload) {
                            Ok(Telemetry {
                                time,
                                variant: Some(data),
                            }) => {
                                let node_telemetry = NodeTelemetry {
                                    node_key: mesh_packet.from,
                                    datetime: time.timestamp_to_datetime().unwrap_or(Utc::now()),
                                    variant: data.clone(),
                                };

                                self.app_event_tx
                                    .send(AppEvent::TelemetryArrived(node_telemetry.clone()))?;

                                self.state_action_tx
                                    .send(StateAction::NodeLastTelemetrySet(node_telemetry))?;
                            }
                            Ok(_) => {
                                tracing::debug!("TelemetryApp with empty variant");
                            }
                            Err(e) => {
                                tracing::debug!("can't decode TelemetryApp payload: {:?}", e);
                            }
                        },
                        _ => {}
                    },
                    _ => {}
                }

                self.send_node_update_last_heard(&mesh_packet)?;
            }
            from_radio::PayloadVariant::ClientNotification(notification) => {
                self.state_action_tx
                    .send(StateAction::Toast(match notification.level() {
                        log_record::Level::Warning => Toast::warning(notification.message),
                        log_record::Level::Error | log_record::Level::Critical => Toast::error(notification.message),
                        _ => Toast::normal(notification.message),
                    }))?;
            }
            _ => {}
        }

        Ok(())
    }

    fn send_node_update_last_heard(&self, packet: &MeshPacket) -> anyhow::Result<()> {
        self.state_action_tx.send(StateAction::NodeUpdateLastHeard {
            node_key: packet.from,
            hops: packet.hop_start.saturating_sub(packet.hop_limit),
            snr: packet.rx_snr,
            rssi: packet.rx_rssi,
        })?;

        self.update_online_nodes()?;

        Ok(())
    }

    fn update_online_nodes(&self) -> anyhow::Result<()> {
        let state = &self.state_rx.borrow();
        let now = Utc::now();

        let count = state.nodes.iter().fold(0, |mut counter, (_, node)| {
            if let Some(last_heard) = node.last_heard
                && (now - last_heard).num_seconds() < ONLINE_NODE_THRESHOLD_SECS
            {
                counter += 1;
            }

            counter
        });

        self.state_action_tx.send(StateAction::NodesOnlineSet(count))?;

        Ok(())
    }
}
