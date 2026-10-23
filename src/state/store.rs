use chrono::Utc;
use meshtastic::protobufs::{config, module_config, routing, telemetry};
use std::collections::hash_map::Entry;
use std::time::{Duration, Instant};
use tokio::{
    sync::{broadcast, mpsc, watch},
    time,
};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::service::update_nodeinfo_telemetry;
use crate::types::{
    Chat, Message, NodeLastTelemetry, NodeLastTelemetryItem, Traceroute, TracerouteIndicatorState, TracerouteState,
};
use crate::{
    state::{State, StateAction},
    types::{ConnectionState, DeviceDiscoveringState, FormItemKey, SettingsFormState, Tab},
};

const TICK_INTERVAL_MILLIS: u64 = 33;
const RX_INDICATOR_TIMEOUT_MILLIS: u128 = 250;
const DB_INDICATOR_TIMEOUT_MILLIS: u128 = 250;
const TOAST_QUICK_TIMEOUT_MILLIS: u128 = 500;
const SPLASH_LOGO_TIMEOUT_MILLIS: u128 = 1500;

pub struct Store {
    state_action_rx: mpsc::UnboundedReceiver<StateAction>,
    state_tx: watch::Sender<State>,
    stored_action_tx: broadcast::Sender<StateAction>,
}

impl Store {
    pub fn new(
        initial_state: State,
    ) -> (
        Self,
        mpsc::UnboundedSender<StateAction>,
        watch::Receiver<State>,
        broadcast::Receiver<StateAction>,
    ) {
        let (state_action_tx, state_action_rx) = mpsc::unbounded_channel::<StateAction>();
        let (state_tx, state_rx) = watch::channel(initial_state);
        let (stored_action_tx, stored_action_rx) = broadcast::channel::<StateAction>(1000);

        (
            Self {
                state_action_rx,
                state_tx,
                stored_action_tx,
            },
            state_action_tx,
            state_rx,
            stored_action_rx,
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
        let stored_action = action.clone();

        let is_changed: bool = match action {
            StateAction::SplashLogo => {
                self.state_tx.send_modify(|state| {
                    state.splash_logo = true;
                    state.splash_logo_t = Instant::now();
                });

                true
            }
            StateAction::AppConfigApply(cfg) => {
                self.state_tx.send_modify(|state| {
                    state.active_tab = cfg.active_tab;
                    state.active_device = cfg.active_device;
                    state.devices = cfg.devices;
                    state.nodes_sort_by = cfg.nodes_sort_by;
                    state.nodes_filter = cfg.nodes_filter;
                    state.ui_config = cfg.ui_config;

                    state.update_nodes_view();
                });

                true
            }
            StateAction::TabSwitchToNext => {
                self.state_tx.send_modify(|state| {
                    state.active_tab = state.active_tab.next();
                });

                true
            }
            StateAction::TabSwitchToPrevious => {
                self.state_tx.send_modify(|state| {
                    state.active_tab = state.active_tab.prev();
                });

                true
            }
            StateAction::DeviceActiveSet(device) => {
                self.state_tx.send_modify(|state| {
                    state.active_device = Some(device);
                });

                true
            }
            StateAction::ConnectionStart => {
                self.state_tx.send_modify(|state| {
                    state.connection_state = ConnectionState::Connecting;
                    state.connection_attempt += 1;
                    state.reconnection_backoff = None;

                    tracing::debug!("connection attempt #{}", state.connection_attempt);
                });

                true
            }
            StateAction::ConnectionFail(error) => {
                self.state_tx.send_modify(|state| {
                    state.connection_state = ConnectionState::ProblemDetected {
                        since: Instant::now(),
                        error,
                    };

                    state.device_config = Default::default();
                    state.device_module_config = Default::default();
                    state.nodes_loader_current = 0;
                    state.nodes_loader_total = 0;
                    state.settings_form_data = None;
                    state.settings_form_is_changed = false;
                    state.settings_form_original_data = None;
                    state.settings_form_state = Default::default();
                });

                true
            }
            StateAction::ConnectionStop => {
                self.state_tx.send_modify(|state| {
                    state.active_chat = None;
                    state.active_device = None;
                    state.active_traceroutes.clear();
                    state.channels = State::default_channels();
                    state.chats.clear();
                    state.connection_attempt = 0;
                    state.connection_state = Default::default();
                    state.device_config = Default::default();
                    state.device_module_config = Default::default();
                    state.messages.clear();
                    state.my_node_key = None;
                    state.nodeinfo_key = None;
                    state.nodeinfo_telemetry.clear();
                    state.nodes_loader_current = 0;
                    state.nodes_loader_total = 0;
                    state.nodes_view.clear();
                    state.nodes.clear();
                    state.online_nodes = 0;
                    state.reconnection_backoff = None;
                    state.settings_form_data = None;
                    state.settings_form_is_changed = false;
                    state.settings_form_original_data = None;
                    state.settings_form_state = Default::default();
                    state.traceroutes.clear();

                    state.update_nodes_view();
                });

                true
            }
            StateAction::ConnectionLoadConfig => {
                self.state_tx.send_modify(|state| {
                    state.connection_state = ConnectionState::LoadingConfig;
                });

                true
            }
            StateAction::ConnectionSuccess => {
                self.state_tx.send_modify(|state| {
                    state.connection_state = ConnectionState::Connected;
                    state.connection_attempt = 0;
                    state.reconnection_backoff = None;
                });

                true
            }
            StateAction::DbIndicatorTrigger => {
                self.state_tx.send_modify(|state| {
                    state.db_indicator = true;
                    state.db_indicator_t = Instant::now();
                });

                true
            }
            StateAction::DbNodesLoad(nodes) => {
                self.state_tx.send_modify(|state| {
                    for node in nodes {
                        if state.is_my_node(node.key) {
                            continue;
                        }

                        match state.nodes.entry(node.key) {
                            Entry::Vacant(entry) => {
                                entry.insert(node);
                            }
                            Entry::Occupied(mut entry) => {
                                if node.last_heard > entry.get().last_heard {
                                    entry.insert(node);
                                }
                            }
                        }
                    }

                    state.update_nodes_view();
                });

                true
            }
            StateAction::DbMessagesLoad(messages) => {
                self.state_tx.send_modify(|state| {
                    for message in messages {
                        add_message(message, state);
                    }
                });

                true
            }
            StateAction::DbTracerouteLoad(traceroutes) => {
                self.state_tx.send_modify(|state| {
                    for t in traceroutes {
                        add_node_traceroute(t.node_key, t.message_id, state);

                        state.traceroutes.insert(t.message_id, t);
                    }
                });

                true
            }
            StateAction::ReconnectionBackoffSet(duration) => {
                self.state_tx.send_modify(|state| {
                    state.reconnection_backoff = Some(duration);
                });

                true
            }
            StateAction::LogRecordAdd(r) => {
                self.state_tx.send_modify(|state| {
                    state.logs.push(r);
                });

                true
            }
            StateAction::DevicesDiscoveredAdd(device) => {
                self.state_tx.send_modify(|state| {
                    state.devices_discovered.replace(device);
                });

                true
            }
            StateAction::DeviceDiscoveringStart => {
                self.state_tx.send_modify(|state| {
                    state.device_discovering_state = DeviceDiscoveringState::Scanning;
                    state.devices_discovered.clear();
                });

                true
            }
            StateAction::DeviceDiscoveringDone => {
                self.state_tx.send_modify(|state| {
                    state.device_discovering_state = DeviceDiscoveringState::Finished;
                });

                true
            }
            StateAction::DevicesAdd(device) => {
                self.state_tx.send_modify(|state| {
                    state.devices.replace(device);
                });

                true
            }
            StateAction::DevicesRemove(device) => self.state_tx.send_if_modified(|state| state.devices.remove(&device)),
            StateAction::DeviceConfigSet(variant) => {
                self.state_tx.send_modify(|state| match variant {
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
                });

                true
            }
            StateAction::DeviceModuleConfigSet(variant) => {
                self.state_tx.send_modify(|state| match variant {
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
                });

                true
            }
            StateAction::DeviceMetadataSet(metadata) => {
                self.state_tx.send_modify(|state| {
                    state.device_metadata = Some(metadata);
                });

                true
            }
            StateAction::DeviceCannedMessagesSet(messages) => {
                self.state_tx.send_modify(|state| {
                    state.device_canned_messages = Some(messages);
                });

                true
            }
            StateAction::NodesLoaderIncrement => {
                self.state_tx.send_modify(|state| {
                    state.nodes_loader_current += 1;
                });

                true
            }
            StateAction::NodesLoaderTotalSet(cap) => {
                self.state_tx.send_modify(|state| {
                    state.nodes_loader_total = cap;
                });

                true
            }
            StateAction::NodeInit(node) => self
                .state_tx
                .send_if_modified(|state| match state.nodes.entry(node.key) {
                    Entry::Vacant(entry) => {
                        entry.insert(node);

                        state.update_nodes_view();

                        true
                    }
                    Entry::Occupied(mut entry) => {
                        if node.user.is_some() && node.last_heard > entry.get().last_heard {
                            entry.insert(node);

                            state.update_nodes_view();

                            return true;
                        }

                        false
                    }
                }),
            StateAction::NodeInfoKeySet(node_key) => {
                self.state_tx.send_modify(|state| {
                    state.nodeinfo_key = Some(node_key);

                    update_nodeinfo_telemetry(state);
                });

                true
            }
            StateAction::NodeInfoKeyUnset => {
                self.state_tx.send_modify(|state| {
                    state.nodeinfo_key = None;
                    state.nodeinfo_telemetry.clear();
                    state.nodeinfo_position.clear();
                });

                true
            }
            StateAction::NodeInfoPositionSet(items) => {
                self.state_tx.send_modify(|state| {
                    state.nodeinfo_position = items;
                });

                true
            }
            StateAction::NodeUpdate(node) => self.state_tx.send_if_modified(|state| {
                let Some(existing_node) = state.nodes.get_mut(&node.key) else {
                    return false;
                };

                existing_node.user = node.user;
                existing_node.hops = node.hops;
                existing_node.last_heard = node.last_heard;
                existing_node.snr = node.snr;
                existing_node.rssi = node.rssi;
                existing_node.update_fulltext();

                state.update_nodes_view();

                true
            }),
            StateAction::NodeUpdateLastHeard {
                node_key,
                hops,
                snr,
                rssi,
            } => self.state_tx.send_if_modified(|state| {
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

                state.update_nodes_view();

                true
            }),
            StateAction::NodeDelete(node_key) => self.state_tx.send_if_modified(|state| {
                if state.nodes.remove(&node_key).is_none() {
                    return false;
                }

                state.nodes_last_telemetry.remove(&node_key);

                if let Some(nodeinfo_key) = state.nodeinfo_key
                    && nodeinfo_key == node_key
                {
                    state.nodeinfo_key = None;
                    state.nodeinfo_telemetry.clear();
                }

                state.update_nodes_view();

                true
            }),
            StateAction::NodeOwnerSet(user) => self.state_tx.send_if_modified(|state| {
                let Some(my_node_key) = state.my_node_key else {
                    tracing::warn!("node owner not set: my node key is unknown");
                    return false;
                };

                if let Some(node) = state.nodes.get_mut(&my_node_key) {
                    node.user = Some(user);

                    state.update_nodes_view();

                    return true;
                }

                false
            }),
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
                                .is_none_or(|m| node_telemetry.datetime > m.datetime)
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
                                .is_none_or(|m| node_telemetry.datetime > m.datetime)
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
                                .is_none_or(|m| node_telemetry.datetime > m.datetime)
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
                                .is_none_or(|m| node_telemetry.datetime > m.datetime)
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
                                .is_none_or(|m| node_telemetry.datetime > m.datetime)
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
                                .is_none_or(|m| node_telemetry.datetime > m.datetime)
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
                                .is_none_or(|m| node_telemetry.datetime > m.datetime)
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
                                .is_none_or(|m| node_telemetry.datetime > m.datetime)
                            {
                                telemetry.traffic_management_stats = Some(NodeLastTelemetryItem {
                                    data: metrics,
                                    datetime: node_telemetry.datetime,
                                });
                            }
                        }
                    };

                    update_nodeinfo_telemetry(state);
                });

                true
            }
            StateAction::NodeLastPositionSet(node_position) => {
                self.state_tx.send_modify(|state| {
                    state.nodes_last_position.insert(node_position.node_key, node_position);
                });

                true
            }
            StateAction::ChannelSet(key, channel) => {
                self.state_tx.send_modify(|state| {
                    let chat = Chat::Channel(key);

                    if channel.role.is_disabled() {
                        state.chats.remove(&chat);

                        if state.active_chat.as_ref() == Some(&chat) {
                            state.active_chat = None;
                        }
                    } else if !state.chats.contains_key(&chat) {
                        state.chats.insert_sorted(chat, Vec::new());
                    }

                    state.channels.insert(key, channel);
                });

                true
            }
            StateAction::ActiveChatSet(chat) => {
                self.state_tx.send_modify(|state| {
                    state.active_chat = Some(chat);
                });

                true
            }
            StateAction::ActiveChatUnset => {
                self.state_tx.send_modify(|state| {
                    state.active_chat = None;
                });

                true
            }
            StateAction::ChatPurge(chat) => self.state_tx.send_if_modified(|state| {
                let message_ids: Vec<u32> = match chat {
                    Chat::Channel(channel_key) => state
                        .messages
                        .iter()
                        .filter_map(|(id, message)| (message.channel == channel_key).then_some(*id))
                        .collect(),
                    Chat::Direct(node_key) => {
                        let Some(my_node_key) = state.my_node_key else {
                            tracing::warn!("chat not purged: my node key is unknown");
                            return false;
                        };

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
                            .map(|ch| ch.role.is_disabled())
                            .unwrap_or(false)
                        {
                            state.chats.remove(&chat);
                        } else {
                            if let Some(messages) = state.chats.get_mut(&chat) {
                                messages.clear()
                            }
                        }
                    }
                    Chat::Direct(_) => {
                        state.chats.remove(&chat);
                    }
                };

                true
            }),
            StateAction::RxIndicatorTrigger => {
                self.state_tx.send_modify(|state| {
                    state.rx_indicator_t = Instant::now();
                    state.rx_indicator = true;
                });

                true
            }
            StateAction::NodesSortBySet(sort_by) => {
                self.state_tx.send_modify(|state| {
                    state.nodes_sort_by = sort_by;
                    state.update_nodes_view();
                });

                true
            }
            StateAction::NodesFilterSet(filter) => {
                self.state_tx.send_modify(|state| {
                    state.nodes_filter = filter.to_lowercase();
                    state.update_nodes_view();
                });

                true
            }
            StateAction::NodesOnlineSet(count) => self.state_tx.send_if_modified(|state| {
                if state.online_nodes == count {
                    return false;
                }

                state.online_nodes = count;

                true
            }),
            StateAction::MyNodeKeySet(number) => {
                self.state_tx.send_modify(|state| {
                    state.my_node_key = Some(number);
                    state.update_nodes_view();
                });

                true
            }
            StateAction::DirectChatStart(node_key) => {
                self.state_tx.send_modify(|state| {
                    let chat = Chat::Direct(node_key);

                    if !state.chats.contains_key(&chat) {
                        state.chats.insert_sorted(chat.clone(), Vec::new());
                    }

                    state.nodeinfo_key = None;
                    state.active_chat = Some(chat);
                    state.active_tab = Tab::Chat;
                });

                true
            }
            StateAction::MessageAdd(message) => {
                self.state_tx.send_modify(|state| add_message(message, state));

                true
            }
            StateAction::MessageErrorSet {
                message_id,
                error,
                datetime,
            } => self.state_tx.send_if_modified(|state| {
                if let Some(message) = state.messages.get_mut(&message_id) {
                    message.routing_error = error;

                    return true;
                }

                if let Some(traceroute) = state.traceroutes.get_mut(&message_id) {
                    traceroute.routing_error = error;
                    traceroute.duration = datetime - traceroute.datetime;

                    if !matches!(error, Some(routing::Error::None)) {
                        traceroute.state = TracerouteState::RoutingError;

                        state.active_traceroutes.remove(&message_id);
                        state.traceroute_indicator = TracerouteIndicatorState::Error;
                        state.traceroute_indicator_t = Instant::now();
                    }

                    return true;
                }

                false
            }),
            StateAction::Toast(toast) => {
                tracing::info!("TOAST kind=\"{:?}\" text=\"{}\"", toast.kind, toast.text);

                self.state_tx.send_modify(|state| {
                    state.toast_queue.push_back(toast);
                });

                true
            }
            StateAction::SettingsFormLoadingStart { id } => {
                self.state_tx.send_modify(|state| {
                    state.settings_form_original_data = None;
                    state.settings_form_data = None;
                    state.settings_form_is_changed = false;
                    state.settings_form_state = SettingsFormState::Loading { id };
                });

                true
            }
            StateAction::SettingsFormLoadingFail { id, error } => {
                self.state_tx.send_modify(|state| {
                    state.settings_form_original_data = None;
                    state.settings_form_data = None;
                    state.settings_form_is_changed = false;
                    state.settings_form_state = SettingsFormState::LoadingFailed { id, error };
                });

                true
            }
            StateAction::SettingsFormLoadingDone { id, data } => {
                self.state_tx.send_modify(|state| {
                    state.settings_form_original_data = Some(data.clone());
                    state.settings_form_data = Some(data);
                    state.settings_form_is_changed = false;
                    state.settings_form_state = SettingsFormState::Loaded { id };
                });

                true
            }
            StateAction::SettingsFormSavingStart { id } => {
                self.state_tx.send_modify(|state| {
                    state.settings_form_state = SettingsFormState::Saving { id };
                });

                true
            }
            StateAction::SettingsFormSavingFailed { id } => {
                self.state_tx.send_modify(|state| {
                    state.settings_form_state = SettingsFormState::Loaded { id };
                });

                true
            }
            StateAction::SettingsFormClose => {
                self.state_tx.send_modify(|state| {
                    state.settings_form_original_data = None;
                    state.settings_form_data = None;
                    state.settings_form_is_changed = false;
                    state.settings_form_state = SettingsFormState::Inactive;
                });

                true
            }
            StateAction::SettingsFormReset => {
                self.state_tx.send_modify(|state| {
                    state.settings_form_data = state.settings_form_original_data.clone();
                    state.settings_form_is_changed = false;
                });

                true
            }
            StateAction::SettingsFormValueSet { key, value } => self.state_tx.send_if_modified(|state| {
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

                true
            }),
            StateAction::UiConfigSet { config } => {
                self.state_tx.send_modify(|state| {
                    state.ui_config = config;
                });

                true
            }
            StateAction::TracerouteStart {
                message_id,
                node_key,
                datetime,
            } => {
                self.state_tx.send_modify(|state| {
                    state.traceroutes.insert(
                        message_id,
                        Traceroute {
                            message_id,
                            node_key,
                            datetime,
                            ..Default::default()
                        },
                    );

                    add_node_traceroute(node_key, message_id, state);

                    state.active_traceroutes.insert(message_id);
                });

                true
            }
            StateAction::TracerouteTimeout { message_id } => {
                self.state_tx.send_modify(|state| {
                    state.traceroutes.entry(message_id).and_modify(|t| {
                        t.state = TracerouteState::TimedOut;
                        t.duration = Utc::now().signed_duration_since(t.datetime);
                    });

                    state.active_traceroutes.remove(&message_id);
                    state.traceroute_indicator = TracerouteIndicatorState::Error;
                    state.traceroute_indicator_t = Instant::now();
                });

                true
            }
            StateAction::TracerouteFinish {
                message_id,
                datetime,
                route_towards,
                route_back,
            } => {
                let mut is_changed = false;

                self.state_tx.send_modify(|state| {
                    state.traceroutes.entry(message_id).and_modify(|t| {
                        t.duration = datetime - t.datetime;
                        t.route_towards = route_towards;
                        t.route_back = route_back;
                        t.state = TracerouteState::Finished;

                        is_changed = true;
                    });

                    if state.active_traceroutes.remove(&message_id) {
                        is_changed = true;
                    }

                    if is_changed {
                        state.traceroute_indicator = TracerouteIndicatorState::Success;
                        state.traceroute_indicator_t = Instant::now();
                    }
                });

                is_changed
            }
            StateAction::TracerouteRemove { message_id } => {
                self.state_tx.send_modify(|state| {
                    if let Some(traceroute) = state.traceroutes.remove(&message_id) {
                        state
                            .nodes_traceroutes
                            .entry(traceroute.node_key)
                            .and_modify(|nt| nt.retain(|id| *id != message_id));
                    }

                    state.active_traceroutes.remove(&message_id);
                    state.traceroute_indicator = TracerouteIndicatorState::Off;
                });

                true
            }
        };

        if is_changed {
            let _ = self.stored_action_tx.send(stored_action);
        }

        Ok(())
    }

    fn handle_tick(&mut self) -> anyhow::Result<()> {
        self.state_tx.send_if_modified(|state| {
            let mut is_changed = false;

            if state.rx_indicator && state.rx_indicator_t.elapsed().as_millis() > RX_INDICATOR_TIMEOUT_MILLIS {
                state.rx_indicator = false;

                is_changed = true;
            }

            if state.db_indicator && state.db_indicator_t.elapsed().as_millis() > DB_INDICATOR_TIMEOUT_MILLIS {
                state.db_indicator = false;

                is_changed = true;
            }

            if !state.active_traceroutes.is_empty()
                && state.traceroute_indicator == TracerouteIndicatorState::Off
                && state.traceroute_indicator_t.elapsed().as_millis() > TracerouteIndicatorState::Running as u128
            {
                state.traceroute_indicator = TracerouteIndicatorState::Running;
                state.traceroute_indicator_t = Instant::now();

                is_changed = true;
            }

            if state.traceroute_indicator != TracerouteIndicatorState::Off
                && state.traceroute_indicator_t.elapsed().as_millis() > state.traceroute_indicator as u128
            {
                state.traceroute_indicator = TracerouteIndicatorState::Off;
                state.traceroute_indicator_t = Instant::now();

                is_changed = true;
            }

            if state.splash_logo && state.splash_logo_t.elapsed().as_millis() > SPLASH_LOGO_TIMEOUT_MILLIS {
                state.splash_logo = false;

                is_changed = true;
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

                    is_changed = true;
                }
            }

            if state.toast.is_none() && !state.toast_queue.is_empty() {
                state.toast = state.toast_queue.pop_front();
                state.toast_t = Instant::now();

                is_changed = true;
            }

            is_changed
        });

        Ok(())
    }
}

fn add_message(message: Message, state: &mut State) {
    if state.messages.contains_key(&message.id) {
        return;
    }

    if message.is_emoji {
        if let Some(reply_message) = state.messages.get_mut(&message.reply_message_id) {
            reply_message.reactions.push(message.id);
        }

        state.messages.insert(message.id, message);

        return;
    }

    let Some(my_node_key) = state.my_node_key else {
        tracing::warn!(message_id = message.id, "message skipped: my node key is unknown");
        return;
    };

    let chat = Chat::from((&message, my_node_key));

    if let Some(chat_messages) = state.chats.get_mut(&chat) {
        chat_messages.push(message.id);
    } else {
        state.chats.insert_sorted(chat, vec![message.id]);
    }

    state.messages.insert(message.id, message);
}

fn add_node_traceroute(node_key: u32, message_id: u32, state: &mut State) {
    let node_traceroutes = state.nodes_traceroutes.entry(node_key).or_default();

    if !node_traceroutes.contains(&message_id) {
        node_traceroutes.push(message_id);
    }
}
