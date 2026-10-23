use crate::service::ONLINE_NODE_THRESHOLD_SECS;
use crate::types::*;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use ordermap::OrderMap;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
pub struct State {
    pub active_chat: Option<Chat>,
    pub active_device: Option<Device>,
    pub active_tab: Tab,
    pub active_traceroutes: HashSet<u32>,
    pub channels: HashMap<u32, Channel>,
    pub chats: OrderMap<Chat, Vec<u32>>,
    pub connection_attempt: u32,
    pub connection_state: ConnectionState,
    pub db_indicator_t: Instant,
    pub db_indicator: bool,
    pub device_canned_messages: Option<String>,
    pub device_config: DeviceConfig,
    pub device_discovering_state: DeviceDiscoveringState,
    pub device_metadata: Option<meshtastic::protobufs::DeviceMetadata>,
    pub device_module_config: DeviceModuleConfig,
    pub devices_discovered: BTreeSet<Device>,
    pub devices: BTreeSet<Device>,
    pub logs: Vec<LogRecord>,
    pub messages: HashMap<u32, Message>,
    pub my_node_key: Option<u32>,
    pub nodeinfo_telemetry: Vec<TelemetryItem>,
    pub nodeinfo_position: Vec<NodePosition>,
    pub nodeinfo_key: Option<u32>,
    pub nodes_filter: String,
    pub nodes_last_telemetry: HashMap<u32, NodeLastTelemetry>,
    pub nodes_last_position: HashMap<u32, NodePosition>,
    pub nodes_loader_current: u32,
    pub nodes_loader_total: u32,
    pub nodes_sort_by: NodesSortBy,
    pub nodes_traceroutes: HashMap<u32, Vec<u32>>,
    pub nodes_view: Vec<u32>,
    pub nodes: HashMap<u32, Node>,
    pub online_nodes: u32,
    pub reconnection_backoff: Option<Duration>,
    pub rx_indicator_t: Instant,
    pub rx_indicator: bool,
    pub settings_form_data: Option<FormData>,
    pub settings_form_is_changed: bool,
    pub settings_form_original_data: Option<FormData>,
    pub settings_form_state: SettingsFormState,
    pub splash_logo_t: Instant,
    pub splash_logo: bool,
    pub toast_queue: VecDeque<Toast>,
    pub toast_t: Instant,
    pub toast: Option<Toast>,
    pub traceroute_indicator_t: Instant,
    pub traceroute_indicator: TracerouteIndicatorState,
    pub traceroutes: HashMap<u32, Traceroute>,
    pub ui_config: UiConfig,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active_chat: None,
            active_device: None,
            active_tab: Tab::default(),
            active_traceroutes: HashSet::with_capacity(10),
            channels: State::default_channels(),
            chats: OrderMap::with_capacity(8),
            connection_attempt: 0,
            connection_state: ConnectionState::default(),
            db_indicator: false,
            db_indicator_t: Instant::now(),
            device_discovering_state: DeviceDiscoveringState::default(),
            device_config: DeviceConfig::default(),
            device_metadata: None,
            device_module_config: DeviceModuleConfig::default(),
            device_canned_messages: None,
            devices_discovered: BTreeSet::default(),
            devices: BTreeSet::default(),
            logs: Vec::with_capacity(1000),
            messages: HashMap::with_capacity(1000),
            my_node_key: None,
            nodeinfo_key: None,
            nodeinfo_telemetry: Vec::default(),
            nodeinfo_position: Vec::default(),
            nodes_traceroutes: HashMap::with_capacity(200),
            nodes_last_telemetry: HashMap::with_capacity(200),
            nodes_last_position: HashMap::with_capacity(200),
            nodes_sort_by: NodesSortBy::default(),
            nodes_filter: String::default(),
            nodes_view: Vec::with_capacity(1000),
            nodes: HashMap::with_capacity(1000),
            nodes_loader_current: 0,
            nodes_loader_total: 0,
            online_nodes: 0,
            reconnection_backoff: None,
            rx_indicator_t: Instant::now(),
            rx_indicator: false,
            settings_form_state: SettingsFormState::default(),
            settings_form_original_data: None,
            settings_form_data: None,
            settings_form_is_changed: false,
            splash_logo_t: Instant::now(),
            splash_logo: false,
            toast_queue: VecDeque::default(),
            toast_t: Instant::now(),
            toast: None,
            traceroutes: HashMap::with_capacity(400),
            traceroute_indicator: TracerouteIndicatorState::Off,
            traceroute_indicator_t: Instant::now(),
            ui_config: UiConfig::default(),
        }
    }
}

impl State {
    pub fn is_my_node(&self, node_key: u32) -> bool {
        self.my_node_key == Some(node_key)
    }

    pub fn get_my_node(&self) -> Option<&Node> {
        self.my_node_key.and_then(|key| self.nodes.get(&key))
    }

    pub fn update_nodes_view(&mut self) {
        let filter: Vec<&str> = self
            .nodes_filter
            .split_whitespace()
            .map(|t| t.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let now = Utc::now();

        self.nodes_view = self
            .nodes
            .values()
            .filter(|node| {
                if filter.is_empty() {
                    return true;
                }

                let online_token = if node
                    .last_heard
                    .map(|last_heard| (now - last_heard).num_seconds() < ONLINE_NODE_THRESHOLD_SECS)
                    .unwrap_or(false)
                {
                    "$online"
                } else {
                    "$offline"
                };

                let traceroute_token = if self
                    .nodes_traceroutes
                    .get(&node.key)
                    .and_then(|nt| Some(!nt.is_empty()))
                    .unwrap_or(false)
                {
                    "$traceroute"
                } else {
                    ""
                };

                let telemetry_token = if self.nodes_last_telemetry.contains_key(&node.key) {
                    "$telemetry"
                } else {
                    ""
                };

                let position_token = if self.nodes_last_position.contains_key(&node.key) {
                    "$position"
                } else {
                    ""
                };

                filter.iter().all(|token| {
                    node.fulltext.contains(token)
                        || online_token.contains(token)
                        || traceroute_token.contains(token)
                        || telemetry_token.contains(token)
                        || position_token.contains(token)
                })
            })
            .sorted_by(|n1, n2| {
                match (Some(n1.key) == self.my_node_key, Some(n2.key) == self.my_node_key) {
                    (true, true) => return Ordering::Equal,
                    (false, true) => return Ordering::Greater,
                    (true, false) => return Ordering::Less,
                    _ => {}
                };

                match &self.nodes_sort_by {
                    NodesSortBy::Hops => n1
                        .hops
                        .unwrap_or(u32::MAX)
                        .cmp(&n2.hops.unwrap_or(u32::MAX))
                        .then(n1.snr.total_cmp(&n2.snr).reverse()),
                    NodesSortBy::LastHeard => n1
                        .last_heard
                        .unwrap_or(DateTime::default())
                        .cmp(&n2.last_heard.unwrap_or(DateTime::default()))
                        .reverse(),
                    NodesSortBy::ShortName => n1.short_name().cmp(&n2.short_name()),
                    NodesSortBy::LongName => n1.long_name().cmp(&n2.long_name()),
                    NodesSortBy::HwModel => n1
                        .hw_model()
                        .cmp(&n2.hw_model())
                        .then(n1.short_name().cmp(&n2.short_name())),
                    NodesSortBy::Role => n1.role().cmp(&n2.role()).then(
                        n1.hops
                            .unwrap_or(u32::MAX)
                            .cmp(&n2.hops.unwrap_or(u32::MAX))
                            .then(n1.snr.total_cmp(&n2.snr).reverse()),
                    ),
                }
            })
            .map(|node| node.key)
            .collect();
    }

    pub fn default_channels() -> HashMap<u32, Channel> {
        (0..8).map(|i| (i, Channel::disabled(i))).collect()
    }
}
