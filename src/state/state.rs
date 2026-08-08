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
    pub channels: OrderMap<u32, Channel>,
    pub chats: OrderMap<Chat, Vec<u32>>,
    pub connection_attempt: u32,
    pub connection_state: ConnectionState,
    pub config_loaded: bool,
    pub device_config: DeviceConfig,
    pub device_discovering_state: DeviceDiscoveringState,
    pub device_metadata: Option<meshtastic::protobufs::DeviceMetadata>,
    pub device_module_config: DeviceModuleConfig,
    pub device_canned_messages: Option<String>,
    pub devices_discovered: BTreeSet<Device>,
    pub devices: BTreeSet<Device>,
    pub logs: Vec<LogRecord>,
    pub messages: HashMap<u32, Message>,
    pub my_node_key: Option<u32>,
    pub my_node_user_hash: u64,
    pub nodeinfo: Option<u32>,
    pub nodeinfo_telemetry: Vec<TelemetryItem>,
    pub nodes: HashMap<u32, Node>,
    pub nodes_traceroutes: HashMap<u32, Vec<u32>>,
    pub nodes_stash: Vec<Node>,
    pub nodes_stash_cap: u32,
    pub nodes_last_telemetry: HashMap<u32, NodeLastTelemetry>,
    pub nodes_sort_by: NodesSortBy,
    pub nodes_filter: String,
    pub nodes_view: Vec<u32>,
    pub online_nodes: u16,
    pub reconnection_backoff: Option<Duration>,
    pub rx: bool,
    pub rx_t: Instant,
    pub settings_form_data: Option<FormData>,
    pub settings_form_is_changed: bool,
    pub settings_form_original_data: Option<FormData>,
    pub settings_form_state: SettingsFormState,
    pub splash_logo: bool,
    pub splash_logo_t: Instant,
    pub toast: Option<Toast>,
    pub toast_queue: VecDeque<Toast>,
    pub toast_t: Instant,
    pub traceroutes: OrderMap<u32, Traceroute>,
    pub traceroute_t: Instant,
    pub ui_config: UiConfig,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active_chat: Default::default(),
            active_device: Default::default(),
            active_tab: Default::default(),
            active_traceroutes: Default::default(),
            channels: OrderMap::with_capacity(8),
            chats: Default::default(),
            connection_attempt: Default::default(),
            connection_state: Default::default(),
            config_loaded: Default::default(),
            device_discovering_state: Default::default(),
            device_config: Default::default(),
            device_metadata: Default::default(),
            device_module_config: Default::default(),
            device_canned_messages: Default::default(),
            devices_discovered: Default::default(),
            devices: Default::default(),
            logs: Vec::with_capacity(1000),
            messages: Default::default(),
            my_node_key: Default::default(),
            my_node_user_hash: Default::default(),
            nodeinfo: Default::default(),
            nodeinfo_telemetry: Default::default(),
            nodes_traceroutes: Default::default(),
            nodes_last_telemetry: Default::default(),
            nodes_sort_by: Default::default(),
            nodes_filter: Default::default(),
            nodes_view: Vec::with_capacity(1000),
            nodes: HashMap::with_capacity(1000),
            nodes_stash: Default::default(),
            nodes_stash_cap: Default::default(),
            online_nodes: Default::default(),
            reconnection_backoff: Default::default(),
            rx_t: Instant::now(),
            rx: Default::default(),
            settings_form_state: Default::default(),
            settings_form_original_data: Default::default(),
            settings_form_data: Default::default(),
            settings_form_is_changed: Default::default(),
            splash_logo_t: Instant::now(),
            splash_logo: Default::default(),
            toast_queue: Default::default(),
            toast_t: Instant::now(),
            toast: Default::default(),
            traceroutes: Default::default(),
            traceroute_t: Instant::now(),
            ui_config: Default::default(),
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
                    .last_heard.map(|last_heard| (now - last_heard).num_seconds() < ONLINE_NODE_THRESHOLD_SECS)
                    .unwrap_or(false)
                {
                    "$online"
                } else {
                    "$offline"
                };

                let telemetry_token = if self.nodes_last_telemetry.contains_key(&node.key) {
                    "$telemetry"
                } else {
                    ""
                };

                filter.iter().all(|token| {
                    node.fulltext.contains(token) || online_token.contains(token) || telemetry_token.contains(token)
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
}
