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
    meshtastic::types::{MeshtasticCommand, MeshtasticEvent},
    state::{State, StateAction},
    types::{AppEvent, Node},
};

const TICK_INTERVAL_MILLIS: u64 = 1000;
const ONLINE_THRESHOLD_MINUTES: i64 = 120;

pub struct NodesService {
    app_event_tx: broadcast::Sender<AppEvent>,
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    meshtastic_command_tx: mpsc::UnboundedSender<MeshtasticCommand>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
}

impl NodesService {
    pub fn new(
        app_event_tx: broadcast::Sender<AppEvent>,
        app_event_rx: broadcast::Receiver<AppEvent>,
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        meshtastic_command_tx: mpsc::UnboundedSender<MeshtasticCommand>,
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
        let mut tick_interval = time::interval(Duration::from_millis(TICK_INTERVAL_MILLIS));

        loop {
            tokio::select! {
                Ok(event) = self.app_event_rx.recv() => self.handle_app_event(event),
                Ok(event) = self.meshtastic_event_rx.recv() => self.handle_meshtastic_event(event),
                _ = tick_interval.tick() => self.handle_tick(),
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
            PayloadVariant::NodeInfo(node_info) => {
                match Node::try_from(&node_info) {
                    Ok(node) => self
                        .state_action_tx
                        .send(StateAction::AddNode(node))
                        .unwrap_or_log(),
                    Err(e) => {
                        tracing::debug!(
                            node_id = node_info.num,
                            "can't convert NodeInfo into Node: {}",
                            e
                        );
                    }
                };
            }
            _ => {}
        }
    }

    fn handle_tick(&mut self) {
        let state = &self.state_rx.borrow();
        let now = Utc::now();

        let online_nodes: u16 = state.nodes.iter().fold(0, |mut counter, (_, node)| {
            if let Some(last_heard) = node.last_heard
                && (now - last_heard).num_minutes() > ONLINE_THRESHOLD_MINUTES
            {
                counter += 1;
            }

            counter
        });

        self.state_action_tx
            .send(StateAction::SetOnlineNodes(online_nodes))
            .unwrap_or_log();
    }
}
