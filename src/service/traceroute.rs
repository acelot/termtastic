use std::{ops::Sub, time::Duration};

use chrono::Utc;
use itertools::Itertools;
use meshtastic::{
    Message as _,
    protobufs::{
        PortNum, RouteDiscovery, from_radio,
        mesh_packet::{self, TransportMechanism},
    },
};
use tokio::{
    sync::{broadcast, mpsc, watch},
    time,
};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::{
    meshtastic::types::{CommandToMeshtastic, MeshtasticEvent},
    state::StateAction,
    types::AppEvent,
};
use crate::{state::State, types::TracerouteState};
use crate::{
    types::{Toast, TracerouteItem},
    ui::helpers::DateTimeExt,
};

pub const TRACEROUTE_TIMEOUT_SECS: i64 = 60;

const CHECK_TRACEROUTE_INTERVAL_SECS: u64 = 1;
const TRACEROUTE_COOLDOWN_SECS: i64 = 30;

pub struct TracerouteService {
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
}

impl TracerouteService {
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
        let mut check_traceroutes_interval = time::interval(Duration::from_secs(CHECK_TRACEROUTE_INTERVAL_SECS));

        loop {
            tokio::select! {
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

    fn handle_app_event(&self, event: Result<AppEvent, broadcast::error::RecvError>) -> anyhow::Result<()> {
        let state = &self.state_rx.borrow();

        match event {
            Ok(app_event) => match app_event {
                AppEvent::TracerouteRequested(node_key) => {
                    let last_run = state
                        .traceroutes
                        .first().map(|(_, t)| t.datetime)
                        .unwrap_or_default();

                    let seconds_left = TRACEROUTE_COOLDOWN_SECS - Utc::now().sub(last_run).num_seconds();

                    if seconds_left > 0 {
                        self.state_action_tx.send(StateAction::Toast(Toast::warning(format!(
                            "traceroute cooldown: wait {} secs...",
                            seconds_left
                        ))))?;
                    } else {
                        let my_node_num = state.my_node_key.expect("should be Some");

                        self.meshtastic_command_tx.send(CommandToMeshtastic::RunTraceroute {
                            node_num: node_key,
                            my_node_num,
                        })?;
                    }
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
                    self.state_action_tx
                        .send(StateAction::Toast(Toast::normal("traceroute started")))?;
                }
                MeshtasticEvent::TracerouteFailed(e) => {
                    tracing::error!("traceroute failed: {:?}", e);

                    self.state_action_tx
                        .send(StateAction::Toast(Toast::error("traceroute failed")))?;
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
                                    datetime: mesh_packet.rx_time.timestamp_to_datetime().unwrap_or(Utc::now()),
                                })?;
                            } else {
                                let route_towards = route_discovery
                                    .snr_towards
                                    .into_iter()
                                    .map(TracerouteItem::Snr)
                                    .interleave(route_discovery.route.into_iter().map(TracerouteItem::Node))
                                    .collect();

                                let route_back = route_discovery
                                    .snr_back
                                    .into_iter()
                                    .map(TracerouteItem::Snr)
                                    .interleave(route_discovery.route_back.into_iter().map(TracerouteItem::Node))
                                    .collect();

                                self.state_action_tx.send(StateAction::TracerouteFinish {
                                    message_id: data.request_id,
                                    datetime: mesh_packet.rx_time.timestamp_to_datetime().unwrap_or(Utc::now()),
                                    route_towards,
                                    route_back,
                                })?;

                                self.state_action_tx
                                    .send(StateAction::Toast(Toast::success("traceroute finished")))?;
                            }
                        }
                        Err(e) => {
                            tracing::debug!("can't decode TracerouteApp payload: {:?}", e);
                        }
                    },
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }

        Ok(())
    }

    fn check_traceroutes(&self) -> anyhow::Result<()> {
        let state = &self.state_rx.borrow();

        if !state.active_traceroutes.is_empty() {
            self.state_action_tx.send(StateAction::TracerouteTrigger)?;
        }

        for message_id in state.active_traceroutes.iter() {
            if let Some(traceroute) = state.traceroutes.get(message_id) {
                if traceroute.state != TracerouteState::Started {
                    continue;
                }

                if Utc::now().signed_duration_since(traceroute.datetime).num_seconds() < TRACEROUTE_TIMEOUT_SECS {
                    continue;
                }
            };

            self.state_action_tx.send(StateAction::TracerouteTimeout {
                message_id: *message_id,
            })?;

            self.state_action_tx
                .send(StateAction::Toast(Toast::error("traceroute timed out")))?;
        }

        Ok(())
    }
}
