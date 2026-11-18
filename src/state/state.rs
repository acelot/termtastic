use crate::types::*;
use meshtastic::protobufs::DeviceMetrics;
use ordermap::{OrderMap, OrderSet};
use std::{
    collections::{BTreeSet, HashMap, HashSet, VecDeque},
    time::{Duration, Instant},
};
use uuid::Uuid;

pub const MAX_LOG_RECORDS: usize = 1000;

#[derive(Debug, Clone)]
pub struct State {
    pub active_chat: Option<Chat>,
    pub active_device: Option<Device>,
    pub active_tab: Tab,
    pub active_traceroutes: HashSet<u32>,
    pub channels: HashMap<u32, Channel>,
    pub chats: OrderMap<Chat, ChatState>,
    pub connection_attempt: u32,
    pub connection_state: ConnectionState,
    pub db_load_state: DbLoadState,
    pub db_load_transaction_nodes: Option<Vec<Node>>,
    pub db_load_transaction_nodes_last_telemetry: Option<Vec<NodeTelemetry>>,
    pub db_load_transaction_nodes_last_position: Option<HashMap<u32, NodePosition>>,
    pub db_load_transaction_messages: Option<Vec<Message>>,
    pub db_load_transaction_traceroutes: Option<Vec<Traceroute>>,
    pub db_load_transaction_my_node_device_metrics: Option<Vec<NodeTelemetryTyped<DeviceMetrics>>>,
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
    pub messages: HashMap<Uuid, Message>,
    pub messages_is_loading: bool,
    pub my_node_key: Option<u32>,
    pub my_node_device_metrics: VecDeque<NodeTelemetryTyped<DeviceMetrics>>,
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
    pub packets_id_map: HashMap<u32, Uuid>,
    pub reconnection_backoff: Option<Duration>,
    pub rx_indicator_t: Instant,
    pub rx_indicator: bool,
    pub settings_form_data: Option<FormData>,
    pub settings_form_is_changed: bool,
    pub settings_form_original_data: Option<FormData>,
    pub settings_form_state: SettingsFormState,
    pub splash_logo_t: Instant,
    pub splash_logo: bool,
    pub terminal_title: Option<String>,
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
            db_load_state: DbLoadState::default(),
            db_load_transaction_nodes: None,
            db_load_transaction_nodes_last_telemetry: None,
            db_load_transaction_nodes_last_position: None,
            db_load_transaction_messages: None,
            db_load_transaction_traceroutes: None,
            db_load_transaction_my_node_device_metrics: None,
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
            messages_is_loading: false,
            my_node_key: None,
            my_node_device_metrics: VecDeque::default(),
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
            packets_id_map: HashMap::with_capacity(1000),
            reconnection_backoff: None,
            rx_indicator_t: Instant::now(),
            rx_indicator: false,
            settings_form_state: SettingsFormState::default(),
            settings_form_original_data: None,
            settings_form_data: None,
            settings_form_is_changed: false,
            splash_logo_t: Instant::now(),
            splash_logo: false,
            terminal_title: None,
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
        (0..=7).map(|i| (i, Channel::disabled(i))).collect()
    }

    pub fn unread_messages_count_by_chat(&self, chat: Chat) -> usize {
        let Some(chat_state) = self.chats.get(&chat) else {
            return 0;
        };

        chat_state
            .max_id()
            .and_then(|max_id| chat_state.ids.get_index_of(&max_id))
            .and_then(|index| chat_state.ids.len().checked_sub(index + 1))
            .unwrap_or(0)
    }

    pub fn unread_messages_count(&self) -> usize {
        self.chats
            .keys()
            .map(|ch| self.unread_messages_count_by_chat(*ch))
            .sum()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChatState {
    pub ids: OrderSet<Uuid>,
    selected_id: Option<Uuid>,
    max_id: Option<Uuid>,
}

impl ChatState {
    pub fn new<I>(ids: I, selected_id: Option<Uuid>) -> Self
    where
        I: IntoIterator<Item = Uuid>,
    {
        Self {
            ids: OrderSet::from_iter(ids),
            selected_id,
            max_id: selected_id,
        }
    }

    pub fn select(&mut self, id: Uuid) -> bool {
        let mut changed = false;

        if self.selected_id != Some(id) {
            self.selected_id = Some(id);
            changed = true;
        }

        if Some(id) > self.max_id() {
            self.max_id = Some(id);
            changed = true;
        }

        changed
    }

    pub fn select_last(&mut self) -> bool {
        if let Some(last_id) = self.ids.last() {
            return self.select(last_id.clone());
        }

        false
    }

    pub fn selected_id(&self) -> Option<Uuid> {
        self.selected_id
    }

    pub fn max_id(&self) -> Option<Uuid> {
        self.max_id
    }

    pub fn clear(&mut self) {
        self.ids = Default::default();
        self.selected_id = None;
        self.max_id = None;
    }
}
