use std::cmp::Ordering;
use std::time::Duration;

use chrono::{DateTime, Utc};
use itertools::Itertools;
use meshtastic::protobufs::Position;
use meshtastic::protobufs::mesh_packet::TransportMechanism;
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

use crate::types::{NodePosition, NodeTelemetry, NodesSortBy, Toast};
use crate::{
    meshtastic::types::{CommandToMeshtastic, MeshtasticEvent},
    state::StateAction,
    types::{AppEvent, Node},
};
use crate::{state::State, ui::helpers::DateTimeExt};

pub const ONLINE_NODE_THRESHOLD_SECS: i64 = 7200;
const UPDATE_ONLINE_NODES_INTERVAL_SECS: u64 = 2;

pub struct NodesService {
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
}

impl NodesService {
    pub fn new(
        app_event_rx: broadcast::Receiver<AppEvent>,
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
        meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
    ) -> Self {
        Self {
            app_event_rx,
            state_rx,
            state_action_tx,
            meshtastic_command_tx,
            meshtastic_event_rx,
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
                AppEvent::DirectChatRequested(node_key) => {
                    let _ = self.state_action_tx.send(StateAction::DirectChatStart(node_key));
                }
                AppEvent::NodesSortByPrevRequested => {
                    let _ = self
                        .state_action_tx
                        .send(StateAction::NodesSortBySet(state.nodes_sort_by.prev()));
                }
                AppEvent::NodesSortByNextRequested => {
                    let _ = self
                        .state_action_tx
                        .send(StateAction::NodesSortBySet(state.nodes_sort_by.next()));
                }
                AppEvent::NodesFilterChanged(filter) => {
                    let _ = self.state_action_tx.send(StateAction::NodesFilterSet(filter));
                }
                AppEvent::NodeInfoBroadcastRequested => {
                    let my_node = state.get_my_node().expect("should be Some");

                    let user = match my_node.try_into() {
                        Ok(user) => user,
                        Err(e) => {
                            tracing::error!("NodeInfo broadcast failed: {}", e);

                            let _ = self
                                .state_action_tx
                                .send(StateAction::Toast(Toast::error("NodeInfo broadcast failed")));

                            return Ok(());
                        }
                    };

                    let _ = self
                        .meshtastic_command_tx
                        .send(CommandToMeshtastic::BroadcastNodeInfo { channel_id: 0, user });
                }
                AppEvent::NodeInfoPopupOpenRequested(node_key) => {
                    let _ = self.state_action_tx.send(StateAction::NodeInfoKeySet(node_key));
                }
                AppEvent::NodeInfoPopupCloseRequested => {
                    let _ = self.state_action_tx.send(StateAction::NodeInfoKeyUnset);
                }
                AppEvent::NodeDeleteRequested(node_key) => {
                    let my_node_num = state.my_node_key.expect("should be Some");

                    let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::DeleteNode {
                        node_num: node_key,
                        my_node_num,
                    });
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
                MeshtasticEvent::NodeInfoBroadcastSent => {
                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::success("NodeInfo broadcast sent")));
                }
                MeshtasticEvent::NodeInfoBroadcastFailed(e) => {
                    tracing::error!("NodeInfo broadcast failed: {:?}", e);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::error("NodeInfo broadcast failed")));
                }
                MeshtasticEvent::NodeRemoveAccepted => {
                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::success("node removed")));
                }
                MeshtasticEvent::NodeRemoveFailed(e) => {
                    tracing::error!("node remove failed: {:?}", e);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::error("node remove failed")));
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
            from_radio::PayloadVariant::NodeInfo(node_info) => {
                let _ = self.state_action_tx.send(StateAction::NodesLoaderIncrement);

                match Node::try_from(&node_info) {
                    Ok(node) => {
                        let node_key = node.key;
                        let last_heard = node.last_heard.unwrap_or(Utc::now());

                        let _ = self.state_action_tx.send(StateAction::NodeInit(node));

                        if let Some(device_metrics) = node_info.device_metrics {
                            let _ = self
                                .state_action_tx
                                .send(StateAction::NodeLastTelemetrySet(NodeTelemetry {
                                    node_key,
                                    datetime: last_heard,
                                    variant: meshtastic::protobufs::telemetry::Variant::DeviceMetrics(device_metrics),
                                }));
                        }
                    }
                    Err(e) => {
                        tracing::debug!(node_key = node_info.num, "can't convert NodeInfo into Node: {}", e);
                    }
                };
            }
            from_radio::PayloadVariant::Packet(mesh_packet) => {
                if mesh_packet.transport_mechanism() != TransportMechanism::TransportInternal {
                    let _ = self.state_action_tx.send(StateAction::NodeInit((&mesh_packet).into()));
                }

                match &mesh_packet.payload_variant {
                    Some(mesh_packet::PayloadVariant::Decoded(data)) => match data.portnum() {
                        PortNum::NodeinfoApp => match User::decode(&*data.payload) {
                            Ok(user) => {
                                match Node::try_from((&mesh_packet, &user)) {
                                    Ok(node) => {
                                        let _ = self.state_action_tx.send(StateAction::NodeUpdate(node));
                                    }
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
                                    let _ = self.state_action_tx.send(StateAction::NodeOwnerSet((&user).into()));
                                }
                                Some(admin_message::PayloadVariant::RemoveByNodenum(node_num)) => {
                                    let _ = self.state_action_tx.send(StateAction::NodeDelete(node_num));
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

                                let _ = self
                                    .state_action_tx
                                    .send(StateAction::NodeLastTelemetrySet(node_telemetry));
                            }
                            Ok(_) => {
                                tracing::debug!("TelemetryApp with empty variant");
                            }
                            Err(e) => {
                                tracing::debug!("can't decode TelemetryApp payload: {:?}", e);
                            }
                        },
                        PortNum::PositionApp => match Position::decode(&*data.payload) {
                            Ok(position) => {
                                let node_position = NodePosition {
                                    node_key: mesh_packet.from,
                                    datetime: mesh_packet.rx_time.timestamp_to_datetime().unwrap_or(Utc::now()),
                                    position,
                                };

                                let _ = self
                                    .state_action_tx
                                    .send(StateAction::NodeLastPositionSet(node_position));
                            }
                            Err(e) => {
                                tracing::debug!("can't decode Position payload: {:?}", e);
                            }
                        },
                        _ => {}
                    },
                    _ => {}
                }

                self.send_node_update_last_heard(&mesh_packet)?;
            }
            from_radio::PayloadVariant::ClientNotification(notification) => {
                let _ = self
                    .state_action_tx
                    .send(StateAction::Toast(match notification.level() {
                        log_record::Level::Warning => Toast::warning(notification.message),
                        log_record::Level::Error | log_record::Level::Critical => Toast::error(notification.message),
                        _ => Toast::normal(notification.message),
                    }));
            }
            _ => {}
        }

        Ok(())
    }

    fn send_node_update_last_heard(&self, packet: &MeshPacket) -> anyhow::Result<()> {
        let _ = self.state_action_tx.send(StateAction::NodeUpdateLastHeard {
            node_key: packet.from,
            hops: packet.hop_start.saturating_sub(packet.hop_limit),
            snr: packet.rx_snr,
            rssi: packet.rx_rssi,
        });

        self.update_online_nodes()?;

        Ok(())
    }

    fn update_online_nodes(&self) -> anyhow::Result<()> {
        let now = Utc::now();

        let count = self.state_rx.borrow().nodes.iter().fold(0, |mut counter, (_, node)| {
            if let Some(last_heard) = node.last_heard
                && (now - last_heard).num_seconds() < ONLINE_NODE_THRESHOLD_SECS
            {
                counter += 1;
            }

            counter
        });

        let _ = self.state_action_tx.send(StateAction::NodesOnlineSet(count));

        Ok(())
    }
}

pub fn update_nodes_view(state: &mut State) {
    let filter: Vec<&str> = state
        .nodes_filter
        .split_whitespace()
        .map(|t| t.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let now = Utc::now();

    state.nodes_view = state
        .nodes
        .values()
        .filter(|node| {
            if filter.is_empty() {
                return true;
            }

            let online_token = if node
                .last_heard
                .map(|last_heard| (now - last_heard).num_seconds() < ONLINE_NODE_THRESHOLD_SECS)
                .unwrap_or(false)
            {
                "$online"
            } else {
                "$offline"
            };

            let traceroute_token = if state
                .nodes_traceroutes
                .get(&node.key)
                .and_then(|nt| Some(!nt.is_empty()))
                .unwrap_or(false)
            {
                "$traceroute"
            } else {
                ""
            };

            let telemetry_token = if state.nodes_last_telemetry.contains_key(&node.key) {
                "$telemetry"
            } else {
                ""
            };

            let position_token = if state.nodes_last_position.contains_key(&node.key) {
                "$position"
            } else {
                ""
            };

            filter.iter().all(|token| {
                node.fulltext.contains(token)
                    || online_token.contains(token)
                    || traceroute_token.contains(token)
                    || telemetry_token.contains(token)
                    || position_token.contains(token)
            })
        })
        .sorted_by(|n1, n2| {
            match (Some(n1.key) == state.my_node_key, Some(n2.key) == state.my_node_key) {
                (true, true) => return Ordering::Equal,
                (false, true) => return Ordering::Greater,
                (true, false) => return Ordering::Less,
                _ => {}
            };

            match &state.nodes_sort_by {
                NodesSortBy::Hops => n1
                    .hops
                    .unwrap_or(u32::MAX)
                    .cmp(&n2.hops.unwrap_or(u32::MAX))
                    .then(n1.snr.total_cmp(&n2.snr).reverse()),
                NodesSortBy::LastHeard => n1
                    .last_heard
                    .unwrap_or(DateTime::default())
                    .cmp(&n2.last_heard.unwrap_or(DateTime::default()))
                    .reverse(),
                NodesSortBy::ShortName => n1.short_name().cmp(&n2.short_name()),
                NodesSortBy::LongName => n1.long_name().cmp(&n2.long_name()),
                NodesSortBy::HwModel => n1
                    .hw_model()
                    .cmp(&n2.hw_model())
                    .then(n1.short_name().cmp(&n2.short_name())),
                NodesSortBy::Role => n1.role().cmp(&n2.role()).then(
                    n1.hops
                        .unwrap_or(u32::MAX)
                        .cmp(&n2.hops.unwrap_or(u32::MAX))
                        .then(n1.snr.total_cmp(&n2.snr).reverse()),
                ),
            }
        })
        .map(|node| node.key)
        .collect();
}
