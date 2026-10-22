use std::time::Duration;

use chrono::Utc;
use itertools::Itertools;
use meshtastic::{
    Message as _,
    protobufs::{
        PortNum, RouteDiscovery, from_radio, log_record,
        mesh_packet::{self, TransportMechanism},
    },
};
use tokio::{
    sync::{broadcast, mpsc, watch},
    time,
};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::types::{Toast, TracerouteItem};
use crate::{
    meshtastic::types::{CommandToMeshtastic, MeshtasticEvent},
    state::StateAction,
    types::{AppEvent, UNKNOWN_NODE},
};
use crate::{state::State, types::TracerouteState};

const CHECK_TRACEROUTE_INTERVAL_MILLIS: u64 = 500;
const TRACEROUTE_TIMEOUT_SECS: i64 = 60;
const TRACEROUTE_SNR_MULTIPLIER: f32 = 4.0;

pub struct TracerouteService {
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    stored_action_rx: broadcast::Receiver<StateAction>,
    meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
}

impl TracerouteService {
    pub fn new(
        app_event_rx: broadcast::Receiver<AppEvent>,
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        stored_action_rx: broadcast::Receiver<StateAction>,
        meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
        meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
    ) -> Self {
        Self {
            app_event_rx,
            state_rx,
            state_action_tx,
            stored_action_rx,
            meshtastic_command_tx,
            meshtastic_event_rx,
        }
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        let mut check_traceroutes_interval = time::interval(Duration::from_millis(CHECK_TRACEROUTE_INTERVAL_MILLIS));

        loop {
            tokio::select! {
                event = self.stored_action_rx.recv() => self.handle_stored_action(event)?,
                event = self.app_event_rx.recv() => self.handle_app_event(event)?,
                event = self.meshtastic_event_rx.recv() => self.handle_meshtastic_event(event)?,
                _ = check_traceroutes_interval.tick() => self.check_traceroutes()?,
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_stored_action(&self, event: Result<StateAction, broadcast::error::RecvError>) -> anyhow::Result<()> {
        match event {
            Ok(StateAction::TracerouteFinish { message_id, .. }) => {
                let state = self.state_rx.borrow();

                let Some(traceroute) = state.traceroutes.get(&message_id) else {
                    return Ok(());
                };

                let node_short_name = state
                    .nodes
                    .get(&traceroute.node_key)
                    .unwrap_or(&UNKNOWN_NODE)
                    .short_name();

                self.state_action_tx.send(StateAction::Toast(Toast::success(format!(
                    "[{}] traceroute finished",
                    node_short_name
                ))))?;
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("broadcast receiver lagged by {} events", n);
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_app_event(&self, event: Result<AppEvent, broadcast::error::RecvError>) -> anyhow::Result<()> {
        match event {
            Ok(app_event) => match app_event {
                AppEvent::TracerouteRequested(node_key) => {
                    let my_node_key = self.state_rx.borrow().my_node_key.expect("should be Some");

                    if node_key == my_node_key {
                        let _ = self
                            .state_action_tx
                            .send(StateAction::Toast(Toast::warning("can't traceroute yourself")));

                        return Ok(());
                    }

                    self.meshtastic_command_tx.send(CommandToMeshtastic::RunTraceroute {
                        node_num: node_key,
                        my_node_num: my_node_key,
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
                MeshtasticEvent::TracerouteStarted => {
                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::normal("traceroute started")));
                }
                MeshtasticEvent::TracerouteFailed(e) => {
                    tracing::error!("traceroute failed: {:?}", e);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::error("traceroute failed")));
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
            from_radio::PayloadVariant::Packet(mesh_packet) => match &mesh_packet.payload_variant {
                Some(mesh_packet::PayloadVariant::Decoded(data)) => match data.portnum() {
                    PortNum::TracerouteApp => match RouteDiscovery::decode(&*data.payload) {
                        Ok(route_discovery) => {
                            if mesh_packet.transport_mechanism() == TransportMechanism::TransportInternal {
                                self.state_action_tx.send(StateAction::TracerouteStart {
                                    node_key: mesh_packet.to,
                                    message_id: mesh_packet.id,
                                    datetime: Utc::now(),
                                })?;

                                return Ok(());
                            };

                            let route_towards: Vec<TracerouteItem> = route_discovery
                                .snr_towards
                                .into_iter()
                                .map(|snr| TracerouteItem::Snr(snr as f32 / TRACEROUTE_SNR_MULTIPLIER))
                                .interleave(route_discovery.route.into_iter().map(TracerouteItem::Node))
                                .collect();

                            let route_back: Vec<TracerouteItem> = route_discovery
                                .snr_back
                                .into_iter()
                                .map(|snr| TracerouteItem::Snr(snr as f32 / TRACEROUTE_SNR_MULTIPLIER))
                                .interleave(route_discovery.route_back.into_iter().map(TracerouteItem::Node))
                                .collect();

                            self.state_action_tx.send(StateAction::TracerouteFinish {
                                message_id: data.request_id,
                                datetime: Utc::now(),
                                route_towards,
                                route_back,
                            })?;
                        }
                        Err(e) => {
                            tracing::debug!("can't decode TracerouteApp payload: {:?}", e);
                        }
                    },
                    _ => {}
                },
                _ => {}
            },
            from_radio::PayloadVariant::ClientNotification(notification) => {
                if let Some(message_id) = notification.reply_id
                    && notification.level() == log_record::Level::Warning
                    && notification.message.to_lowercase().contains("traceroute")
                {
                    let _ = self.state_action_tx.send(StateAction::TracerouteRemove { message_id });
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn check_traceroutes(&self) -> anyhow::Result<()> {
        let state = self.state_rx.borrow();

        for message_id in state.active_traceroutes.iter() {
            if let Some(traceroute) = state.traceroutes.get(message_id) {
                if traceroute.state != TracerouteState::Started {
                    continue;
                }

                if Utc::now().signed_duration_since(traceroute.datetime).num_seconds() < TRACEROUTE_TIMEOUT_SECS {
                    continue;
                }

                let node_short_name = state
                    .nodes
                    .get(&traceroute.node_key)
                    .unwrap_or(&UNKNOWN_NODE)
                    .short_name();

                self.state_action_tx.send(StateAction::Toast(Toast::error(format!(
                    "[{}] traceroute timed out",
                    &node_short_name
                ))))?;
            };

            self.state_action_tx.send(StateAction::TracerouteTimeout {
                message_id: *message_id,
            })?;
        }

        Ok(())
    }
}
