use std::{
    cmp::Ordering,
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
    u32, u128,
};

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

use crate::{
    state::{State, StateAction},
    types::{Channel, ConnectionState, DeviceDiscoveringState, NodesSortBy, Tab},
};

const TICK_INTERVAL_MILLIS: u64 = 33;
const UPDATE_ONLINE_NODES_INTERVAL_SECS: u64 = 15;
const RX_TIMEOUT_MILLIS: u128 = 200;
const TOAST_QUICK_TIMEOUT_MILLIS: u128 = 500;
const SPLASH_LOGO_TIMEOUT_MILLIS: u128 = 2000;
const ONLINE_NODE_THRESHOLD_SECS: i64 = 7200;

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
        let mut online_nodes_interval =
            time::interval(Duration::from_secs(UPDATE_ONLINE_NODES_INTERVAL_SECS));

        loop {
            tokio::select! {
                Some(action) = self.action_rx.recv() => self.handle_action(action)?,
                _ = tick_interval.tick() => self.handle_tick()?,
                _ = online_nodes_interval.tick() => self.update_online_nodes()?,
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_action(&mut self, action: StateAction) -> anyhow::Result<()> {
        let prev_state = self.state.clone();

        match action {
            StateAction::SplashLogo => {
                self.state.splash_logo = true;
                self.state.splash_logo_t = Instant::now();
            }
            StateAction::AppConfigApply(cfg) => {
                self.state.active_tab = cfg.active_tab;
                self.state.active_device = cfg.active_device;
                self.state.tcp_devices = cfg.tcp_devices;
                self.state.nodes_sort_by = cfg.nodes_sort_by;
            }
            StateAction::TabSwitchTo(tab) => {
                self.state.active_tab = tab;
                self.state.need_clear_frame = true;
            }
            StateAction::TabSwitchToNext => {
                self.state.active_tab = self.state.active_tab.next();
                self.state.need_clear_frame = true;
            }
            StateAction::TabSwitchToPrevious => {
                self.state.active_tab = self.state.active_tab.prev();
                self.state.need_clear_frame = true;
            }
            StateAction::DeviceActiveSet(device) => {
                self.state.active_device = Some(device);
            }
            StateAction::ConnectionStart => {
                self.state.connection_state = ConnectionState::Connecting;
                self.state.connection_attempt += 1;
                self.state.reconnection_backoff = None;

                tracing::debug!("connection attempt #{}", self.state.connection_attempt);
            }
            StateAction::ConnectionFail(error) => {
                self.state.connection_state = ConnectionState::ProblemDetected {
                    since: Instant::now(),
                    error,
                };
            }
            StateAction::ConnectionStop => {
                self.state.connection_state = ConnectionState::NotConnected;
                self.state.connection_attempt = 0;
                self.state.reconnection_backoff = None;
                self.state.active_device = None;
                self.state.channels.clear();
                self.state.nodes_sort.clear();
                self.state.nodes.clear();
                self.state.online_nodes = 0;
            }
            StateAction::ConnectionSuccess => {
                self.state.connection_state = ConnectionState::Connected;
                self.state.connection_attempt = 0;
                self.state.reconnection_backoff = None;
            }
            StateAction::ReconnectionBackoffSet(duration) => {
                self.state.reconnection_backoff = Some(duration);
            }
            StateAction::LogRecordAdd(r) => {
                self.state.logs.push(r);
            }
            StateAction::DeviceDiscoveringStart => {
                self.state.device_discovering_state = DeviceDiscoveringState::Discovering;
            }
            StateAction::DeviceDiscoveringFail(error) => {
                self.state.device_discovering_state = DeviceDiscoveringState::Failed(error);
            }
            StateAction::DeviceDiscoveringDone(devices) => {
                self.state.discovered_devices = devices;
                self.state.device_discovering_state = DeviceDiscoveringState::Done;
            }
            StateAction::DevicesAddTcp(hostaddr) => {
                if !self.state.tcp_devices.contains(&hostaddr) {
                    self.state.tcp_devices.push(hostaddr);
                }
            }
            StateAction::DevicesRemoveTcp(hostaddr) => {
                self.state
                    .tcp_devices
                    .iter()
                    .position(|h| h == &hostaddr)
                    .map(|index| self.state.tcp_devices.remove(index));
            }
            StateAction::NodeAdd(mut node) => {
                if let Some(number) = self.state.my_node_key
                    && node.key == number
                {
                    node.my = true;
                }

                self.state.nodes.insert(node.key, node);

                self.update_nodes_sort();
                self.update_online_nodes()?;
            }
            StateAction::ChannelEnsure(key, channel) => {
                self.state.channels.entry(key).or_insert(channel);
            }
            StateAction::ChannelActiveSet(id) => {
                self.state.active_channel_key = Some(id);
            }
            StateAction::ChannelActiveUnset => {
                self.state.active_channel_key = None;
            }
            StateAction::RxTrigger => {
                self.state.rx_t = Instant::now();
                self.state.rx = true;
            }
            StateAction::NodesSortBySet(sort_by) => {
                self.state.nodes_sort_by = sort_by;
                self.update_nodes_sort();
            }
            StateAction::NodeUpdateLastHeard {
                node_key,
                hops,
                snr,
            } => {
                if let Some(node) = self.state.nodes.get_mut(&node_key) {
                    node.last_heard = Some(Utc::now());
                    node.hops_away = Some(hops);

                    if hops == 0 {
                        node.snr = snr;
                    }

                    self.update_nodes_sort();
                    self.update_online_nodes()?;
                }
            }
            StateAction::MyNodeKeySet(number) => {
                self.state.my_node_key = Some(number);

                if let Some(node) = self.state.nodes.get_mut(&number) {
                    node.my = true;
                }
            }
            StateAction::DirectChatStart(node_key) => {
                self.state
                    .channels
                    .entry(node_key)
                    .or_insert(Channel::direct(node_key));

                self.state.active_channel_key = Some(node_key);
                self.state.active_tab = Tab::Chat;
            }
            StateAction::MessageAdd(channel_key, message) => {
                if let Some(messages_vec) = self.state.messages.get_mut(&channel_key) {
                    messages_vec.push_back(message);
                } else {
                    self.state
                        .messages
                        .insert(channel_key, VecDeque::from(vec![message]));
                }
            }
            StateAction::MessageReactionAdd {
                channel_key,
                message_id,
                emoji,
                node_key,
            } => {
                if let Some(message) = self
                    .state
                    .messages
                    .get_mut(&channel_key)
                    .and_then(|messages| messages.iter_mut().find(|msg| msg.id == message_id))
                {
                    message
                        .reactions
                        .entry(emoji)
                        .or_insert_with(HashMap::new)
                        .insert(node_key, Utc::now());
                }
            }
            StateAction::MessageAck(channel_key, message_id) => {
                if let Some(message) = self
                    .state
                    .messages
                    .get_mut(&channel_key)
                    .and_then(|messages| messages.iter_mut().find(|msg| msg.id == message_id))
                {
                    message.acked = true;
                }
            }
            StateAction::FrameCleared => {
                self.state.need_clear_frame = false;
            }
            StateAction::Toast(toast) => {
                self.state.toast_queue.push_back(toast);
            }
        }

        if self.state != prev_state {
            self.state_tx.send(self.state.clone())?;
        }

        Ok(())
    }

    fn handle_tick(&mut self) -> anyhow::Result<()> {
        if self.state.rx && self.state.rx_t.elapsed().as_millis() > RX_TIMEOUT_MILLIS {
            self.state.rx = false;
            self.state_tx.send(self.state.clone())?;
        }

        if self.state.splash_logo
            && self.state.splash_logo_t.elapsed().as_millis() > SPLASH_LOGO_TIMEOUT_MILLIS
        {
            self.state.splash_logo = false;
            self.state_tx.send(self.state.clone())?;
        }

        if let Some(toast) = &self.state.toast {
            // skip toast quickly if there is another in queue
            let timeout = toast
                .kind
                .timeout()
                .min(if self.state.toast_queue.is_empty() {
                    u128::MAX
                } else {
                    TOAST_QUICK_TIMEOUT_MILLIS
                });

            if self.state.toast_t.elapsed().as_millis() > timeout {
                self.state.toast = None;
                self.state_tx.send(self.state.clone())?;
            }
        }

        if !self.state.toast_queue.is_empty() {
            self.state.toast = self.state.toast_queue.pop_front();
            self.state.toast_t = Instant::now();
            self.state_tx.send(self.state.clone())?;
        }

        Ok(())
    }

    fn update_nodes_sort(&mut self) {
        self.state.nodes_sort = self
            .state
            .nodes
            .values()
            .sorted_by(|n1, n2| {
                match (n1.my, n2.my) {
                    (true, true) => return Ordering::Equal,
                    (false, true) => return Ordering::Greater,
                    (true, false) => return Ordering::Less,
                    _ => {}
                };

                match &self.state.nodes_sort_by {
                    NodesSortBy::Hops => n1
                        .hops_away
                        .unwrap_or(u32::MAX)
                        .cmp(&n2.hops_away.unwrap_or(u32::MAX))
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
                            .unwrap_or(u32::MAX)
                            .cmp(&n2.hops_away.unwrap_or(u32::MAX))
                            .then(n1.snr.total_cmp(&n2.snr).reverse()),
                    ),
                }
            })
            .map(|node| node.key)
            .collect();
    }

    fn update_online_nodes(&mut self) -> anyhow::Result<()> {
        let now = Utc::now();

        self.state.online_nodes = self.state.nodes.iter().fold(0, |mut counter, (_, node)| {
            if let Some(last_heard) = node.last_heard
                && (now - last_heard).num_seconds() < ONLINE_NODE_THRESHOLD_SECS
            {
                counter += 1;
            }

            counter
        });

        Ok(())
    }
}
