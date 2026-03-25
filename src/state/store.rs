use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use itertools::Itertools;
use tokio::{
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
        watch,
    },
    time,
};
use tokio_graceful_shutdown::SubsystemHandle;
use tracing_unwrap::ResultExt;

use crate::{
    state::{State, StateAction},
    types::{ConnectionState, DevicesDiscoveringState, NodesSortBy},
};

const TICK_INTERVAL_MILLIS: u64 = 33;

pub struct Store {
    state: State,
    action_rx: UnboundedReceiver<StateAction>,
    state_tx: watch::Sender<State>,
}

impl Store {
    pub fn new(
        initial_state: State,
    ) -> (Self, UnboundedSender<StateAction>, watch::Receiver<State>) {
        let (action_tx, action_rx) = unbounded_channel::<StateAction>();
        let (state_tx, state_rx) = watch::channel(initial_state.clone());

        (
            Self {
                state: initial_state.clone(),
                action_rx,
                state_tx,
            },
            action_tx,
            state_rx,
        )
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        let mut tick_interval = time::interval(Duration::from_millis(TICK_INTERVAL_MILLIS));

        loop {
            tokio::select! {
                Some(action) = self.action_rx.recv() => self.handle_action(action),
                _ = tick_interval.tick() => self.handle_tick(),
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_action(&mut self, action: StateAction) {
        let prev_state = self.state.clone();

        match action {
            StateAction::AppConfigApply(cfg) => {
                self.state.active_device = cfg.active_device;
                self.state.tcp_devices = cfg.tcp_devices;
            }
            StateAction::TabSwitchToNext => {
                self.state.active_tab = self.state.active_tab.next();
            }
            StateAction::TabSwitchToPrevious => {
                self.state.active_tab = self.state.active_tab.prev();
            }
            StateAction::DeviceActiveSet(device) => {
                self.state.active_device = Some(device);
            }
            StateAction::ConnectionStart => {
                self.state.connection_state = ConnectionState::Connecting;
            }
            StateAction::ConnectionFail(error) => {
                self.state.connection_state = ConnectionState::ProblemDetected {
                    since: Instant::now(),
                    error,
                };
            }
            StateAction::ConnectionStop => {
                self.state.connection_state = ConnectionState::NotConnected;
                self.state.active_device = None;
                self.state.channels.clear();
                self.state.nodes_sort.clear();
                self.state.nodes.clear();
                self.state.online_nodes = 0;
            }
            StateAction::ConnectionSuccess => {
                self.state.connection_state = ConnectionState::Connected;
            }
            StateAction::LogRecordAdd(r) => {
                self.state.logs.push(r);
            }
            StateAction::DevicesDiscoveringStart => {
                self.state.device_discovering_state = DevicesDiscoveringState::InProgress;
            }
            StateAction::DevicesDiscoveringFail(error) => {
                self.state.device_discovering_state = DevicesDiscoveringState::Error(error);
            }
            StateAction::DevicesDiscoveringSuccess(devices) => {
                self.state.device_discovering_state = DevicesDiscoveringState::Finished;
                self.state.discovered_devices = devices;
            }
            StateAction::DevicesAddTcp(hostaddr) => {
                if !self.state.tcp_devices.contains(&hostaddr) {
                    self.state.tcp_devices.push(hostaddr);
                }
            }
            StateAction::DevicesRemoveTcp(hostaddr) => {
                let maybe_index = self.state.tcp_devices.iter().position(|h| h == &hostaddr);

                if let Some(index) = maybe_index {
                    self.state.tcp_devices.remove(index);
                }
            }
            StateAction::NodeAdd(node) => {
                self.state.nodes.insert(node.number, node);
                self.fill_nodes_sort();
            }
            StateAction::ChannelAdd(index, channel) => {
                self.state.channels.insert(index, channel);
            }
            StateAction::ChannelActiveSet(id) => {
                self.state.active_channel_id = Some(id);
            }
            StateAction::ChannelActiveUnset => {
                self.state.active_channel_id = None;
            }
            StateAction::OnlineNodesSet(total) => {
                self.state.online_nodes = total;
            }
            StateAction::RxTrigger => {
                self.state.rx_t = Instant::now();
                self.state.rx = true;
            }
            StateAction::NodesSortBySet(sort_by) => {
                self.state.nodes_sort_by = sort_by;
            }
            StateAction::NodeSetLastHeard(number) => {
                if let Some(node) = self.state.nodes.get_mut(&number) {
                    node.last_heard = Some(Utc::now());
                }
            }
            StateAction::NodeSetSnr(number, snr) => {
                if let Some(node) = self.state.nodes.get_mut(&number) {
                    node.snr = snr;
                }
            }
        }

        if self.state != prev_state {
            self.state_tx.send(self.state.clone()).unwrap_or_log();
        }
    }

    fn handle_tick(&mut self) {
        if self.state.rx_t.elapsed().as_millis() > 200 && self.state.rx {
            self.state.rx = false;
            self.state_tx.send(self.state.clone()).unwrap_or_log();
        }
    }

    fn fill_nodes_sort(&mut self) {
        self.state.nodes_sort = self
            .state
            .nodes
            .values()
            .sorted_by(|n1, n2| match &self.state.nodes_sort_by {
                NodesSortBy::Hops => n1
                    .hops_away
                    .unwrap_or(100)
                    .cmp(&n2.hops_away.unwrap_or(100))
                    .then(n1.snr.total_cmp(&n2.snr).reverse()),
                NodesSortBy::LastHeard => n1
                    .last_heard
                    .unwrap_or(DateTime::default())
                    .cmp(&n2.last_heard.unwrap_or(DateTime::default()))
                    .reverse(),
                NodesSortBy::ShortName => n1.short_name.cmp(&n2.short_name),
                NodesSortBy::LongName => n1.long_name.cmp(&n2.long_name),
                NodesSortBy::HwModel => n1
                    .hw_model
                    .cmp(&n2.hw_model)
                    .then(n1.short_name.cmp(&n2.short_name)),
                NodesSortBy::Role => n1.role.cmp(&n2.role).then(
                    n1.hops_away
                        .unwrap_or(100)
                        .cmp(&n2.hops_away.unwrap_or(100))
                        .then(n1.snr.total_cmp(&n2.snr).reverse()),
                ),
            })
            .map(|node| node.number)
            .collect();
    }
}
