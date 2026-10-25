use crate::types::*;
use ordermap::OrderMap;
use std::collections::HashSet;
use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    time::{Duration, Instant},
};

pub const MAX_LOG_RECORDS: usize = 1000;

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
    pub logs: VecDeque<LogRecord>,
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
            logs: VecDeque::with_capacity(MAX_LOG_RECORDS),
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

    pub fn default_channels() -> HashMap<u32, Channel> {
        (0..8).map(|i| (i, Channel::disabled(i))).collect()
    }
}
