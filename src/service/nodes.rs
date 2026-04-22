use std::time::Duration;

use chrono::Utc;
use meshtastic::{
    Message as _,
    protobufs::{MeshPacket, PortNum, User, from_radio::PayloadVariant, mesh_packet},
};
use tokio::{
    sync::{broadcast, mpsc, watch},
    time,
};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::{
    meshtastic::types::{CommandToMeshtastic, MeshtasticEvent},
    state::{State, StateAction},
    types::{AppEvent, Node},
};

const UPDATE_ONLINE_NODES_INTERVAL_SECS: u64 = 2;
const ONLINE_NODE_THRESHOLD_SECS: i64 = 7200;

#[allow(dead_code)]
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
        let mut online_nodes_interval =
            time::interval(Duration::from_secs(UPDATE_ONLINE_NODES_INTERVAL_SECS));

        loop {
            tokio::select! {
                Ok(event) = self.app_event_rx.recv() => self.handle_app_event(event)?,
                Ok(event) = self.meshtastic_event_rx.recv() => self.handle_meshtastic_event(event)?,
                _ = online_nodes_interval.tick() => self.update_online_nodes()?,
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_app_event(&self, event: AppEvent) -> anyhow::Result<()> {
        match event {
            AppEvent::DirectChatRequested(node_key) => {
                self.state_action_tx
                    .send(StateAction::DirectChatStart(node_key))?;
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_meshtastic_event(&mut self, event: MeshtasticEvent) -> anyhow::Result<()> {
        match event {
            MeshtasticEvent::IncomingPacket(packet) => self.handle_meshtastic_packet(packet)?,
            _ => {}
        }

        Ok(())
    }

    fn handle_meshtastic_packet(&mut self, packet: PayloadVariant) -> anyhow::Result<()> {
        match packet {
            PayloadVariant::MyInfo(my_info) => {
                self.local_my_node_num = Some(my_info.my_node_num);

                self.state_action_tx
                    .send(StateAction::MyNodeKeySet(my_info.my_node_num))?;
            }
            PayloadVariant::NodeInfo(node_info) => {
                match Node::try_from(&node_info) {
                    Ok(node) => {
                        self.state_action_tx.send(StateAction::NodeAdd(node))?;
                        self.update_online_nodes()?;
                    }
                    Err(e) => {
                        tracing::debug!(
                            node_key = node_info.num,
                            "can't convert NodeInfo into Node: {}",
                            e
                        );
                    }
                };

                if Some(node_info.num) == self.local_my_node_num {
                    self.state_action_tx.send(StateAction::DeviceUserSet(
                        node_info.user.expect("should be Some"),
                    ))?;
                }
            }
            PayloadVariant::Packet(packet) => match &packet.payload_variant {
                Some(mesh_packet::PayloadVariant::Decoded(data)) => match data.portnum() {
                    PortNum::NodeinfoApp => match User::decode(&*data.payload) {
                        Ok(user) => {
                            match Node::try_from((&packet, &user)) {
                                Ok(node) => {
                                    self.state_action_tx.send(StateAction::NodeAdd(node))?
                                }
                                Err(e) => {
                                    tracing::debug!(
                                        node_key = packet.from,
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
                    _ => {
                        self.send_node_update_last_heard(&packet)?;
                    }
                },
                _ => {
                    self.send_node_update_last_heard(&packet)?;
                }
            },
            _ => {}
        }

        Ok(())
    }

    fn send_node_update_last_heard(&self, packet: &MeshPacket) -> anyhow::Result<()> {
        self.state_action_tx
            .send(StateAction::NodeUpdateLastHeard {
                node_key: packet.from,
                hops: packet.hop_start.saturating_sub(packet.hop_limit),
                snr: packet.rx_snr,
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

        self.state_action_tx
            .send(StateAction::NodesOnlineSet(count))?;

        Ok(())
    }
}
