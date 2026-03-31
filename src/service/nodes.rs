use std::time::Duration;

use chrono::Utc;
use meshtastic::protobufs::from_radio::PayloadVariant;
use tokio::{
    sync::{broadcast, mpsc, watch},
    time,
};
use tokio_graceful_shutdown::SubsystemHandle;
use tracing_unwrap::ResultExt;

use crate::{
    meshtastic::types::{CommandToMeshtastic, MeshtasticEvent},
    state::{State, StateAction},
    types::{AppEvent, Node},
};

const UPDATE_ONLINE_NODES_INTERVAL_MILLIS: u64 = 1000;
const ONLINE_NODE_THRESHOLD_SECS: i64 = 7200;

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
        let mut online_nodes_interval =
            time::interval(Duration::from_millis(UPDATE_ONLINE_NODES_INTERVAL_MILLIS));

        loop {
            tokio::select! {
                Ok(event) = self.app_event_rx.recv() => self.handle_app_event(event),
                Ok(event) = self.meshtastic_event_rx.recv() => self.handle_meshtastic_event(event),
                _ = online_nodes_interval.tick() => self.update_online_nodes(),
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_app_event(&self, event: AppEvent) {}

    fn handle_meshtastic_event(&mut self, event: MeshtasticEvent) {
        match event {
            MeshtasticEvent::IncomingPacket(packet) => self.handle_meshtastic_packet(packet),
            _ => {}
        }
    }

    fn handle_meshtastic_packet(&mut self, packet: PayloadVariant) {
        match packet {
            PayloadVariant::MyInfo(my_info) => {
                self.state_action_tx
                    .send(StateAction::MyNodeKeySet(my_info.my_node_num))
                    .unwrap_or_log();
            }
            PayloadVariant::NodeInfo(node_info) => {
                match Node::try_from(&node_info) {
                    Ok(node) => self
                        .state_action_tx
                        .send(StateAction::NodeAdd(node))
                        .unwrap_or_log(),
                    Err(e) => {
                        tracing::debug!(
                            node_key = node_info.num,
                            "can't convert NodeInfo into Node: {}",
                            e
                        );
                    }
                };
            }
            PayloadVariant::Packet(packet) => {
                self.state_action_tx
                    .send(StateAction::NodeUpdateLastHeard(packet.from))
                    .unwrap_or_log();

                if packet.hop_start == packet.hop_limit {
                    self.state_action_tx
                        .send(StateAction::NodeSetSnr(packet.from, packet.rx_snr))
                        .unwrap_or_log();
                }
            }
            _ => {}
        }
    }

    fn update_online_nodes(&mut self) {
        let state = &self.state_rx.borrow();
        let now = Utc::now();

        let online_nodes: u16 = state.nodes.iter().fold(0, |mut counter, (_, node)| {
            if let Some(last_heard) = node.last_heard
                && (now - last_heard).num_seconds() < ONLINE_NODE_THRESHOLD_SECS
            {
                counter += 1;
            }

            counter
        });

        self.state_action_tx
            .send(StateAction::OnlineNodesSet(online_nodes))
            .unwrap_or_log();
    }
}
