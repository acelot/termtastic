use chrono::Utc;
use meshtastic::protobufs::{config, module_config, routing, telemetry};
use nameof::name_of;
use std::collections::hash_map::Entry;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::{Duration, Instant};
use tokio::{
    sync::{broadcast, mpsc, watch},
    time,
};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::service::update_nodeinfo_telemetry;
use crate::types::{Chat, NodeLastTelemetry, NodeLastTelemetryItem, Traceroute, TracerouteState};
use crate::{
    state::{State, StateAction},
    types::{ConnectionState, DeviceDiscoveringState, FormItemKey, SettingsFormState, Tab},
};

const TICK_INTERVAL_MILLIS: u64 = 33;
const RX_TIMEOUT_MILLIS: u128 = 250;
const TOAST_QUICK_TIMEOUT_MILLIS: u128 = 500;
const SPLASH_LOGO_TIMEOUT_MILLIS: u128 = 1500;

const NODES_VIEW_WATCHLIST: [&'static str; 4] = [
    name_of!(my_node_key in State),
    name_of!(nodes in State),
    name_of!(nodes_filter in State),
    name_of!(nodes_sort_by in State),
];

pub struct Store {
    state_action_rx: mpsc::UnboundedReceiver<StateAction>,
    state_tx: watch::Sender<State>,
    changed_tx: broadcast::Sender<Vec<&'static str>>,
}

impl Store {
    pub fn new(
        initial_state: State,
    ) -> (
        Self,
        mpsc::UnboundedSender<StateAction>,
        watch::Receiver<State>,
        broadcast::Receiver<Vec<&'static str>>,
    ) {
        let (state_action_tx, state_action_rx) = mpsc::unbounded_channel::<StateAction>();
        let (state_tx, state_rx) = watch::channel(initial_state);
        let (changed_tx, changed_rx) = broadcast::channel::<Vec<&'static str>>(1000);

        (
            Self {
                state_action_rx,
                state_tx,
                changed_tx,
            },
            state_action_tx,
            state_rx,
            changed_rx,
        )
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        let mut tick_interval = time::interval(Duration::from_millis(TICK_INTERVAL_MILLIS));

        loop {
            tokio::select! {
                Some(action) = self.state_action_rx.recv() => self.handle_action(action)?,
                _ = tick_interval.tick() => self.handle_tick()?,
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_action(&mut self, action: StateAction) -> anyhow::Result<()> {
        let mut changed: Vec<&'static str> = Vec::new();

        match action {
            StateAction::SplashLogo => {
                self.state_tx.send_modify(|state| {
                    state.splash_logo = true;
                    state.splash_logo_t = Instant::now();

                    changed.extend([name_of!(splash_logo in State), name_of!(splash_logo_t in State)]);
                });
            }
            StateAction::AppConfigApply(cfg) => {
                self.state_tx.send_modify(|state| {
                    state.active_tab = cfg.active_tab;
                    state.active_device = cfg.active_device;
                    state.devices = cfg.devices;
                    state.nodes_sort_by = cfg.nodes_sort_by;
                    state.nodes_filter = cfg.nodes_filter;
                    state.ui_config = cfg.ui_config;
                    state.config_loaded = true;

                    changed.extend([
                        name_of!(active_tab in State),
                        name_of!(active_device in State),
                        name_of!(devices in State),
                        name_of!(nodes_sort_by in State),
                        name_of!(nodes_filter in State),
                        name_of!(ui_config in State),
                        name_of!(config_loaded in State),
                    ]);
                });
            }
            StateAction::TabSwitchToNext => {
                self.state_tx.send_modify(|state| {
                    state.active_tab = state.active_tab.next();

                    changed.push(name_of!(active_tab in State));
                });
            }
            StateAction::TabSwitchToPrevious => {
                self.state_tx.send_modify(|state| {
                    state.active_tab = state.active_tab.prev();

                    changed.push(name_of!(active_tab in State));
                });
            }
            StateAction::DeviceActiveSet(device) => {
                self.state_tx.send_modify(|state| {
                    state.active_device = Some(device);

                    changed.push(name_of!(active_device in State));
                });
            }
            StateAction::ConnectionStart => {
                self.state_tx.send_modify(|state| {
                    state.connection_state = ConnectionState::Connecting;
                    state.connection_attempt += 1;
                    state.reconnection_backoff = None;

                    changed.extend([
                        name_of!(connection_state in State),
                        name_of!(connection_attempt in State),
                        name_of!(reconnection_backoff in State),
                    ]);

                    tracing::debug!("connection attempt #{}", state.connection_attempt);
                });
            }
            StateAction::ConnectionFail(error) => {
                self.state_tx.send_modify(|state| {
                    state.connection_state = ConnectionState::ProblemDetected {
                        since: Instant::now(),
                        error,
                    };

                    state.device_config = Default::default();
                    state.device_module_config = Default::default();
                    state.settings_form_data = None;
                    state.settings_form_is_changed = false;
                    state.settings_form_original_data = None;
                    state.settings_form_state = Default::default();

                    changed.extend([
                        name_of!(connection_state in State),
                        name_of!(device_config in State),
                        name_of!(device_module_config in State),
                        name_of!(settings_form_data in State),
                        name_of!(settings_form_is_changed in State),
                        name_of!(settings_form_original_data in State),
                        name_of!(settings_form_state in State),
                    ]);
                });
            }
            StateAction::ConnectionStop => {
                self.state_tx.send_modify(|state| {
                    state.active_device = None;
                    state.chats.clear();
                    state.channels.clear();
                    state.connection_attempt = 0;
                    state.connection_state = Default::default();
                    state.device_config = Default::default();
                    state.device_module_config = Default::default();
                    state.nodes_view.clear();
                    state.nodes.clear();
                    state.online_nodes = 0;
                    state.reconnection_backoff = None;
                    state.settings_form_data = None;
                    state.settings_form_is_changed = false;
                    state.settings_form_original_data = None;
                    state.settings_form_state = Default::default();

                    changed.extend([
                        name_of!(active_device in State),
                        name_of!(chats in State),
                        name_of!(channels in State),
                        name_of!(connection_attempt in State),
                        name_of!(connection_state in State),
                        name_of!(device_config in State),
                        name_of!(device_module_config in State),
                        name_of!(nodes_view in State),
                        name_of!(nodes in State),
                        name_of!(online_nodes in State),
                        name_of!(reconnection_backoff in State),
                        name_of!(settings_form_data in State),
                        name_of!(settings_form_is_changed in State),
                        name_of!(settings_form_original_data in State),
                        name_of!(settings_form_state in State),
                    ]);
                });
            }
            StateAction::ConnectionLoadConfig => {
                self.state_tx.send_modify(|state| {
                    state.connection_state = ConnectionState::LoadingConfig;

                    changed.push(name_of!(connection_state in State));
                });
            }
            StateAction::ConnectionSuccess => {
                self.state_tx.send_modify(|state| {
                    state.connection_state = ConnectionState::Connected;
                    state.connection_attempt = 0;
                    state.reconnection_backoff = None;

                    changed.extend([
                        name_of!(connection_state in State),
                        name_of!(connection_attempt in State),
                        name_of!(reconnection_backoff in State),
                    ]);
                });
            }
            StateAction::DbNodesLoad(nodes) => {
                self.state_tx.send_modify(|state| {
                    for node in nodes {
                        if let Some(existing_node) = state.nodes.get(&node.key) {
                            if node.last_heard > existing_node.last_heard {
                                state.nodes.insert(node.key, node);
                            }
                        } else {
                            state.nodes.insert(node.key, node);
                        }
                    }

                    changed.extend([name_of!(nodes in State)]);
                });
            }
            StateAction::ReconnectionBackoffSet(duration) => {
                self.state_tx.send_modify(|state| {
                    state.reconnection_backoff = Some(duration);

                    changed.push(name_of!(reconnection_backoff in State));
                });
            }
            StateAction::LogRecordAdd(r) => {
                self.state_tx.send_modify(|state| {
                    state.logs.push(r);

                    changed.push(name_of!(logs in State));
                });
            }
            StateAction::DevicesDiscoveredAdd(device) => {
                self.state_tx.send_modify(|state| {
                    state.devices_discovered.replace(device);

                    changed.push(name_of!(devices_discovered in State));
                });
            }
            StateAction::DeviceDiscoveringStart => {
                self.state_tx.send_modify(|state| {
                    state.device_discovering_state = DeviceDiscoveringState::Scanning;
                    state.devices_discovered.clear();

                    changed.extend([
                        name_of!(device_discovering_state in State),
                        name_of!(devices_discovered in State),
                    ]);
                });
            }
            StateAction::DeviceDiscoveringDone => {
                self.state_tx.send_modify(|state| {
                    state.device_discovering_state = DeviceDiscoveringState::Finished;

                    changed.push(name_of!(device_discovering_state in State));
                });
            }
            StateAction::DevicesAdd(device) => {
                self.state_tx.send_modify(|state| {
                    state.devices.replace(device);

                    changed.push(name_of!(devices in State));
                });
            }
            StateAction::DevicesRemove(device) => {
                self.state_tx.send_if_modified(|state| {
                    if state.devices.remove(&device) {
                        changed.push(name_of!(devices in State));

                        return true;
                    }

                    false
                });
            }
            StateAction::DeviceConfigSet(variant) => {
                self.state_tx.send_modify(|state| {
                    match variant {
                        config::PayloadVariant::Bluetooth(cfg) => {
                            state.device_config.bluetooth = Some(cfg);
                        }
                        config::PayloadVariant::Device(cfg) => {
                            state.device_config.device = Some(cfg);
                        }
                        config::PayloadVariant::DeviceUi(cfg) => {
                            state.device_config.device_ui = Some(cfg);
                        }
                        config::PayloadVariant::Display(cfg) => {
                            state.device_config.display = Some(cfg);
                        }
                        config::PayloadVariant::Lora(cfg) => {
                            state.device_config.lora = Some(cfg);
                        }
                        config::PayloadVariant::Network(cfg) => {
                            state.device_config.network = Some(cfg);
                        }
                        config::PayloadVariant::Position(cfg) => {
                            state.device_config.position = Some(cfg);
                        }
                        config::PayloadVariant::Power(cfg) => {
                            state.device_config.power = Some(cfg);
                        }
                        config::PayloadVariant::Security(cfg) => {
                            state.device_config.security = Some(cfg);
                        }
                        config::PayloadVariant::Sessionkey(cfg) => {
                            state.device_config.sessionkey = Some(cfg);
                        }
                    }

                    changed.push(name_of!(device_config in State));
                });
            }
            StateAction::DeviceModuleConfigSet(variant) => {
                self.state_tx.send_modify(|state| {
                    match variant {
                        module_config::PayloadVariant::AmbientLighting(cfg) => {
                            state.device_module_config.ambient_lighting = Some(cfg);
                        }
                        module_config::PayloadVariant::Audio(cfg) => {
                            state.device_module_config.audio = Some(cfg);
                        }
                        module_config::PayloadVariant::CannedMessage(cfg) => {
                            state.device_module_config.canned_message = Some(cfg);
                        }
                        module_config::PayloadVariant::DetectionSensor(cfg) => {
                            state.device_module_config.detection_sensor = Some(cfg);
                        }
                        module_config::PayloadVariant::ExternalNotification(cfg) => {
                            state.device_module_config.external_notification = Some(cfg);
                        }
                        module_config::PayloadVariant::Mqtt(cfg) => {
                            state.device_module_config.mqtt = Some(cfg);
                        }
                        module_config::PayloadVariant::NeighborInfo(cfg) => {
                            state.device_module_config.neighbor = Some(cfg);
                        }
                        module_config::PayloadVariant::Paxcounter(cfg) => {
                            state.device_module_config.paxcounter = Some(cfg);
                        }
                        module_config::PayloadVariant::RangeTest(cfg) => {
                            state.device_module_config.range_test = Some(cfg);
                        }
                        module_config::PayloadVariant::RemoteHardware(cfg) => {
                            state.device_module_config.remote_hardware = Some(cfg);
                        }
                        module_config::PayloadVariant::Serial(cfg) => {
                            state.device_module_config.serial = Some(cfg);
                        }
                        module_config::PayloadVariant::Statusmessage(cfg) => {
                            state.device_module_config.status_message = Some(cfg);
                        }
                        module_config::PayloadVariant::StoreForward(cfg) => {
                            state.device_module_config.store_forward = Some(cfg);
                        }
                        module_config::PayloadVariant::Telemetry(cfg) => {
                            state.device_module_config.telemetry = Some(cfg);
                        }
                        module_config::PayloadVariant::TrafficManagement(cfg) => {
                            state.device_module_config.traffic_management = Some(cfg);
                        }
                        module_config::PayloadVariant::Tak(cfg) => {
                            state.device_module_config.tak = Some(cfg);
                        }
                    }

                    changed.push(name_of!(device_module_config in State));
                });
            }
            StateAction::DeviceMetadataSet(metadata) => {
                self.state_tx.send_modify(|state| {
                    state.device_metadata = Some(metadata);

                    changed.push(name_of!(device_metadata in State));
                });
            }
            StateAction::DeviceCannedMessagesSet(messages) => {
                self.state_tx.send_modify(|state| {
                    state.device_canned_messages = Some(messages);

                    changed.push(name_of!(device_canned_messages in State));
                });
            }
            StateAction::NodesStashPush(node) => {
                self.state_tx.send_modify(|state| {
                    state.nodes_stash.push(node);

                    changed.push(name_of!(nodes_stash in State));
                });
            }
            StateAction::NodesStashCapSet(cap) => {
                self.state_tx.send_modify(|state| {
                    state.nodes_stash_cap = cap;

                    changed.push(name_of!(nodes_stash_cap in State));
                });
            }
            StateAction::NodesStashFlush => {
                self.state_tx.send_modify(|state| {
                    state.nodes_stash.clear();
                    state.nodes_stash_cap = 0;

                    changed.extend([name_of!(nodes_stash in State), name_of!(nodes_stash_cap in State)]);
                });
            }
            StateAction::NodeInit(node) => {
                self.state_tx
                    .send_if_modified(|state| match state.nodes.entry(node.key) {
                        Entry::Vacant(entry) => {
                            entry.insert(node);

                            changed.push(name_of!(nodes in State));

                            true
                        }
                        Entry::Occupied(mut entry) => {
                            if node.user.is_some() && node.last_heard > entry.get().last_heard {
                                entry.insert(node);

                                changed.push(name_of!(nodes in State));

                                return true;
                            }

                            false
                        }
                    });
            }
            StateAction::NodeInfoSet(node_key) => {
                self.state_tx.send_modify(|state| {
                    state.nodeinfo = Some(node_key);

                    changed.push(name_of!(nodeinfo in State));

                    if update_nodeinfo_telemetry(state) {
                        changed.push(name_of!(nodeinfo_telemetry in State));
                    }
                });
            }
            StateAction::NodeInfoUnset => {
                self.state_tx.send_modify(|state| {
                    state.nodeinfo = None;
                    state.nodeinfo_telemetry.clear();

                    changed.extend([name_of!(nodeinfo in State), name_of!(nodeinfo_telemetry in State)]);
                });
            }
            StateAction::NodeUpdate(node) => {
                self.state_tx.send_if_modified(|state| {
                    let Some(existing_node) = state.nodes.get_mut(&node.key) else {
                        return false;
                    };

                    existing_node.user = node.user;
                    existing_node.hops = node.hops;
                    existing_node.last_heard = node.last_heard;
                    existing_node.snr = node.snr;
                    existing_node.rssi = node.rssi;
                    existing_node.update_fulltext();

                    changed.push(name_of!(nodes in State));

                    true
                });
            }
            StateAction::NodeUpdateLastHeard {
                node_key,
                hops,
                snr,
                rssi,
            } => {
                self.state_tx.send_if_modified(|state| {
                    let Some(node) = state.nodes.get_mut(&node_key) else {
                        return false;
                    };

                    node.last_heard = Some(Utc::now());
                    node.hops = Some(hops);

                    if hops == 0 {
                        node.snr = snr;
                        node.rssi = Some(rssi);
                    }

                    node.update_fulltext();

                    changed.push(name_of!(nodes in State));

                    true
                });
            }
            StateAction::NodeDelete(node_key) => {
                self.state_tx.send_if_modified(|state| {
                    if state.nodes.remove(&node_key).is_none() {
                        return false;
                    }

                    if let Some(nodeinfo_key) = state.nodeinfo
                        && nodeinfo_key == node_key
                    {
                        state.nodeinfo = None;

                        changed.push(name_of!(nodeinfo in State));
                    }

                    changed.push(name_of!(nodes in State));

                    true
                });
            }
            StateAction::NodeOwnerSet(user) => {
                self.state_tx.send_if_modified(|state| {
                    if let Some(node) = state.nodes.get_mut(&state.my_node_key.expect("should be Some")) {
                        state.my_node_user_hash = calculate_hash(&user);
                        node.user = Some(user);

                        changed.extend([name_of!(my_node_user_hash in State), name_of!(nodes in State)]);

                        return true;
                    }

                    false
                });
            }
            StateAction::NodeLastTelemetrySet(node_telemetry) => {
                self.state_tx.send_modify(|state| {
                    let telemetry = state
                        .nodes_last_telemetry
                        .entry(node_telemetry.node_key)
                        .or_insert(NodeLastTelemetry::default());

                    match node_telemetry.variant {
                        telemetry::Variant::DeviceMetrics(metrics) => {
                            if telemetry
                                .device_metrics
                                .as_ref()
                                .map_or(true, |m| node_telemetry.datetime > m.datetime)
                            {
                                telemetry.device_metrics = Some(NodeLastTelemetryItem {
                                    data: metrics,
                                    datetime: node_telemetry.datetime,
                                });
                            }
                        }
                        telemetry::Variant::EnvironmentMetrics(metrics) => {
                            if telemetry
                                .environment_metrics
                                .as_ref()
                                .map_or(true, |m| node_telemetry.datetime > m.datetime)
                            {
                                telemetry.environment_metrics = Some(NodeLastTelemetryItem {
                                    data: metrics,
                                    datetime: node_telemetry.datetime,
                                });
                            }
                        }
                        telemetry::Variant::AirQualityMetrics(metrics) => {
                            if telemetry
                                .air_quality_metrics
                                .as_ref()
                                .map_or(true, |m| node_telemetry.datetime > m.datetime)
                            {
                                telemetry.air_quality_metrics = Some(NodeLastTelemetryItem {
                                    data: metrics,
                                    datetime: node_telemetry.datetime,
                                });
                            }
                        }
                        telemetry::Variant::PowerMetrics(metrics) => {
                            if telemetry
                                .power_metrics
                                .as_ref()
                                .map_or(true, |m| node_telemetry.datetime > m.datetime)
                            {
                                telemetry.power_metrics = Some(NodeLastTelemetryItem {
                                    data: metrics,
                                    datetime: node_telemetry.datetime,
                                });
                            }
                        }
                        telemetry::Variant::LocalStats(metrics) => {
                            if telemetry
                                .local_stats
                                .as_ref()
                                .map_or(true, |m| node_telemetry.datetime > m.datetime)
                            {
                                telemetry.local_stats = Some(NodeLastTelemetryItem {
                                    data: metrics,
                                    datetime: node_telemetry.datetime,
                                });
                            }
                        }
                        telemetry::Variant::HealthMetrics(metrics) => {
                            if telemetry
                                .health_metrics
                                .as_ref()
                                .map_or(true, |m| node_telemetry.datetime > m.datetime)
                            {
                                telemetry.health_metrics = Some(NodeLastTelemetryItem {
                                    data: metrics,
                                    datetime: node_telemetry.datetime,
                                });
                            }
                        }
                        telemetry::Variant::HostMetrics(metrics) => {
                            if telemetry
                                .host_metrics
                                .as_ref()
                                .map_or(true, |m| node_telemetry.datetime > m.datetime)
                            {
                                telemetry.host_metrics = Some(NodeLastTelemetryItem {
                                    data: metrics,
                                    datetime: node_telemetry.datetime,
                                });
                            }
                        }
                        telemetry::Variant::TrafficManagementStats(metrics) => {
                            if telemetry
                                .traffic_management_stats
                                .as_ref()
                                .map_or(true, |m| node_telemetry.datetime > m.datetime)
                            {
                                telemetry.traffic_management_stats = Some(NodeLastTelemetryItem {
                                    data: metrics,
                                    datetime: node_telemetry.datetime,
                                });
                            }
                        }
                    };

                    changed.push(name_of!(nodes_last_telemetry in State));

                    if update_nodeinfo_telemetry(state) {
                        changed.push(name_of!(nodeinfo_telemetry in State));
                    }
                });
            }
            StateAction::ChannelSet(key, channel) => {
                self.state_tx.send_modify(|state| {
                    if !channel.role.is_disabled() {
                        let chat = Chat::Channel(key);

                        if !state.chats.contains_key(&chat) {
                            state.chats.insert_sorted(chat, Vec::new());
                            changed.push(name_of!(chats in State));
                        }
                    }

                    state.channels.insert_sorted(key, channel);

                    changed.push(name_of!(channels in State));
                });
            }
            StateAction::ActiveChatSet(chat) => {
                self.state_tx.send_modify(|state| {
                    state.active_chat = Some(chat);

                    changed.push(name_of!(active_chat in State));
                });
            }
            StateAction::ActiveChatUnset => {
                self.state_tx.send_modify(|state| {
                    state.active_chat = None;

                    changed.push(name_of!(active_chat in State));
                });
            }
            StateAction::ChatPurge(chat) => {
                self.state_tx.send_if_modified(|state| {
                    let message_ids: Vec<u32> = match chat {
                        Chat::Channel(channel_key) => state
                            .messages
                            .iter()
                            .filter_map(|(id, message)| (message.channel == channel_key).then_some(*id))
                            .collect(),
                        Chat::Direct(node_key) => {
                            let my_node_key = state.my_node_key.expect("should be Some");

                            state
                                .messages
                                .iter()
                                .filter_map(|(id, message)| {
                                    ((message.from == my_node_key && message.to == node_key)
                                        || (message.to == my_node_key && message.from == node_key))
                                        .then_some(*id)
                                })
                                .collect()
                        }
                    };

                    for id in message_ids {
                        state.messages.remove(&id);
                    }

                    match chat {
                        Chat::Channel(key) => {
                            if state
                                .channels
                                .get(&key)
                                .and_then(|ch| Some(ch.role.is_disabled()))
                                .unwrap_or(false)
                            {
                                state.chats.remove(&chat);
                            } else {
                                state.chats.get_mut(&chat).map(|messages| messages.clear());
                            }
                        }
                        Chat::Direct(_) => {
                            state.chats.remove(&chat);
                        }
                    };

                    true
                });
            }
            StateAction::RxTrigger => {
                self.state_tx.send_modify(|state| {
                    state.rx_t = Instant::now();
                    state.rx = true;

                    changed.extend([name_of!(rx in State), name_of!(rx_t in State)]);
                });
            }
            StateAction::NodesSortBySet(sort_by) => {
                self.state_tx.send_modify(|state| {
                    state.nodes_sort_by = sort_by;

                    changed.push(name_of!(nodes_sort_by in State));
                });
            }
            StateAction::NodesFilterSet(filter) => {
                self.state_tx.send_modify(|state| {
                    state.nodes_filter = filter.to_lowercase();

                    changed.push(name_of!(nodes_filter in State));
                });
            }
            StateAction::NodesOnlineSet(count) => {
                self.state_tx.send_if_modified(|state| {
                    if state.online_nodes == count {
                        return false;
                    }

                    state.online_nodes = count;

                    changed.push(name_of!(online_nodes in State));

                    true
                });
            }
            StateAction::MyNodeKeySet(number) => {
                self.state_tx.send_modify(|state| {
                    state.my_node_key = Some(number);

                    changed.push(name_of!(my_node_key in State));
                });
            }
            StateAction::DirectChatStart(node_key) => {
                self.state_tx.send_modify(|state| {
                    let chat = Chat::Direct(node_key);

                    if !state.chats.contains_key(&chat) {
                        state.chats.insert_sorted(chat.clone(), Vec::new());
                        changed.push(name_of!(chats in State));
                    }

                    state.active_chat = Some(chat);
                    state.active_tab = Tab::Chat;

                    changed.extend([name_of!(active_chat in State), name_of!(active_tab in State)]);
                });
            }
            StateAction::MessageAdd(message) => {
                self.state_tx.send_modify(|state| {
                    if message.is_emoji {
                        if let Some(reply_message) = state.messages.get_mut(&message.reply_message_id) {
                            reply_message.reactions.push(message.id);
                        }

                        state.messages.insert(message.id, message);

                        changed.push(name_of!(messages in State));

                        return;
                    }

                    let chat = Chat::from((&message, state.my_node_key.expect("should be Some")));

                    if let Some(chat_messages) = state.chats.get_mut(&chat) {
                        chat_messages.push(message.id);
                    } else {
                        state.chats.insert_sorted(chat, vec![message.id]);
                    }

                    state.messages.insert(message.id, message);

                    changed.extend([name_of!(chats in State), name_of!(messages in State)]);
                });
            }
            StateAction::MessageErrorSet {
                message_id,
                error,
                datetime,
            } => {
                self.state_tx.send_if_modified(|state| {
                    if let Some(message) = state.messages.get_mut(&message_id) {
                        message.routing_error = error;

                        changed.push(name_of!(messages in State));

                        return true;
                    }

                    if let Some(traceroute) = state.traceroutes.get_mut(&message_id) {
                        traceroute.routing_error = error;
                        traceroute.duration = datetime - traceroute.datetime;

                        if !matches!(error, Some(routing::Error::None)) {
                            traceroute.state = TracerouteState::RoutingError;
                        }

                        changed.push(name_of!(traceroutes in State));

                        return true;
                    }

                    false
                });
            }
            StateAction::Toast(toast) => {
                self.state_tx.send_modify(|state| {
                    state.toast_queue.push_back(toast);

                    changed.push(name_of!(toast_queue in State));
                });
            }
            StateAction::SettingsFormLoadingStart { id } => {
                self.state_tx.send_modify(|state| {
                    state.settings_form_original_data = None;
                    state.settings_form_data = None;
                    state.settings_form_is_changed = false;
                    state.settings_form_state = SettingsFormState::Loading { id };

                    changed.extend([
                        name_of!(settings_form_original_data in State),
                        name_of!(settings_form_data in State),
                        name_of!(settings_form_is_changed in State),
                        name_of!(settings_form_state in State),
                    ]);
                });
            }
            StateAction::SettingsFormLoadingFail { id, error } => {
                self.state_tx.send_modify(|state| {
                    state.settings_form_original_data = None;
                    state.settings_form_data = None;
                    state.settings_form_is_changed = false;
                    state.settings_form_state = SettingsFormState::LoadingFailed { id, error };

                    changed.extend([
                        name_of!(settings_form_original_data in State),
                        name_of!(settings_form_data in State),
                        name_of!(settings_form_is_changed in State),
                        name_of!(settings_form_state in State),
                    ]);
                });
            }
            StateAction::SettingsFormLoadingDone { id, data } => {
                self.state_tx.send_modify(|state| {
                    state.settings_form_original_data = Some(data.clone());
                    state.settings_form_data = Some(data);
                    state.settings_form_is_changed = false;
                    state.settings_form_state = SettingsFormState::Loaded { id };

                    changed.extend([
                        name_of!(settings_form_original_data in State),
                        name_of!(settings_form_data in State),
                        name_of!(settings_form_is_changed in State),
                        name_of!(settings_form_state in State),
                    ]);
                });
            }
            StateAction::SettingsFormSavingStart { id } => {
                self.state_tx.send_modify(|state| {
                    state.settings_form_state = SettingsFormState::Saving { id };

                    changed.push(name_of!(settings_form_state in State));
                });
            }
            StateAction::SettingsFormSavingFailed { id } => {
                self.state_tx.send_modify(|state| {
                    state.settings_form_state = SettingsFormState::Loaded { id };

                    changed.push(name_of!(settings_form_state in State));
                });
            }
            StateAction::SettingsFormClose => {
                self.state_tx.send_modify(|state| {
                    state.settings_form_original_data = None;
                    state.settings_form_data = None;
                    state.settings_form_is_changed = false;
                    state.settings_form_state = SettingsFormState::Inactive;

                    changed.extend([
                        name_of!(settings_form_original_data in State),
                        name_of!(settings_form_data in State),
                        name_of!(settings_form_is_changed in State),
                        name_of!(settings_form_state in State),
                    ]);
                });
            }
            StateAction::SettingsFormReset => {
                self.state_tx.send_modify(|state| {
                    state.settings_form_data = state.settings_form_original_data.clone();
                    state.settings_form_is_changed = false;

                    changed.extend([
                        name_of!(settings_form_data in State),
                        name_of!(settings_form_is_changed in State),
                    ]);
                });
            }
            StateAction::SettingsFormValueSet { key, value } => {
                self.state_tx.send_if_modified(|state| {
                    let Some(data) = state.settings_form_data.as_mut() else {
                        return false;
                    };

                    match key {
                        FormItemKey::Simple(k) => {
                            data.insert(k.to_owned(), value);
                        }
                        FormItemKey::Custom { setter, .. } => {
                            setter(data, value);
                        }
                        FormItemKey::None => {}
                    }

                    state.settings_form_is_changed = state.settings_form_data != state.settings_form_original_data;

                    changed.extend([
                        name_of!(settings_form_data in State),
                        name_of!(settings_form_is_changed in State),
                    ]);

                    true
                });
            }
            StateAction::UiConfigSet { config } => {
                self.state_tx.send_modify(|state| {
                    state.ui_config = config;

                    changed.push(name_of!(ui_config in State));
                });
            }
            StateAction::TracerouteStart {
                message_id,
                node_key,
                datetime,
            } => {
                self.state_tx.send_modify(|state| {
                    state.traceroutes.insert_sorted(
                        message_id,
                        Traceroute {
                            message_id,
                            node_key,
                            datetime,
                            ..Default::default()
                        },
                    );

                    state
                        .nodes_traceroutes
                        .entry(node_key)
                        .and_modify(|nt| nt.push(message_id))
                        .or_insert(vec![message_id]);

                    changed.extend([name_of!(traceroutes in State), name_of!(nodes_traceroutes in State)]);
                });
            }
            StateAction::TracerouteTimeout { message_id } => {
                self.state_tx.send_if_modified(|state| {
                    let mut is_modified = false;

                    state.traceroutes.entry(message_id).and_modify(|t| {
                        t.state = TracerouteState::TimedOut;

                        changed.push(name_of!(traceroutes in State));
                        is_modified = true;
                    });

                    is_modified
                });
            }
            StateAction::TracerouteFinish {
                message_id,
                datetime,
                route_towards,
                route_back,
            } => {
                self.state_tx.send_if_modified(|state| {
                    let mut is_modified = false;

                    state.traceroutes.entry(message_id).and_modify(|t| {
                        t.duration = datetime - t.datetime;
                        t.route_towards = route_towards;
                        t.route_back = route_back;
                        t.state = TracerouteState::Finished;

                        changed.push(name_of!(traceroutes in State));
                        is_modified = true;
                    });

                    is_modified
                });
            }
        }

        if !changed.is_empty() {
            if changed.iter().any(|field| NODES_VIEW_WATCHLIST.contains(field)) {
                self.state_tx.send_modify(|state| {
                    state.update_nodes_view();

                    changed.push(name_of!(nodes_view in State));
                })
            }

            self.changed_tx.send(changed)?;
        }

        Ok(())
    }

    fn handle_tick(&mut self) -> anyhow::Result<()> {
        let mut changed: Vec<&'static str> = Vec::new();

        self.state_tx.send_if_modified(|state| {
            if state.rx && state.rx_t.elapsed().as_millis() > RX_TIMEOUT_MILLIS {
                state.rx = false;

                changed.push(name_of!(rx in State));
            }

            if state.splash_logo && state.splash_logo_t.elapsed().as_millis() > SPLASH_LOGO_TIMEOUT_MILLIS {
                state.splash_logo = false;

                changed.push(name_of!(splash_logo in State));
            }

            if let Some(toast) = &state.toast {
                // skip toast quickly if there is another in queue
                let timeout = toast.kind.timeout().min(if state.toast_queue.is_empty() {
                    u128::MAX
                } else {
                    TOAST_QUICK_TIMEOUT_MILLIS
                });

                if state.toast_t.elapsed().as_millis() > timeout {
                    state.toast = None;

                    changed.push(name_of!(toast in State));
                }
            }

            if !state.toast_queue.is_empty() {
                state.toast = state.toast_queue.pop_front();
                state.toast_t = Instant::now();

                changed.extend([name_of!(toast in State), name_of!(toast_t in State)]);
            }

            !changed.is_empty()
        });

        if !changed.is_empty() {
            self.changed_tx.send(changed)?;
        }

        Ok(())
    }
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
