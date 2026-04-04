use meshtastic::{
    Message as _,
    protobufs::{MeshPacket, PortNum, User, from_radio::PayloadVariant, mesh_packet},
};
use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::{
    meshtastic::types::{CommandToMeshtastic, MeshtasticEvent},
    state::{State, StateAction},
    types::{AppEvent, Node},
};

#[allow(dead_code)]
pub struct NodesService {
    app_event_tx: broadcast::Sender<AppEvent>,
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
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
        }
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                Ok(event) = self.app_event_rx.recv() => self.handle_app_event(event)?,
                Ok(event) = self.meshtastic_event_rx.recv() => self.handle_meshtastic_event(event)?,
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
                self.state_action_tx
                    .send(StateAction::MyNodeKeySet(my_info.my_node_num))?;
            }
            PayloadVariant::NodeInfo(node_info) => {
                match Node::try_from(&node_info) {
                    Ok(node) => self.state_action_tx.send(StateAction::NodeAdd(node))?,
                    Err(e) => {
                        tracing::debug!(
                            node_key = node_info.num,
                            "can't convert NodeInfo into Node: {}",
                            e
                        );
                    }
                };
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

        Ok(())
    }
}
