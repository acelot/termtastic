use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};

use hostaddr::HostAddr;

use crate::types::*;

#[derive(Debug, Clone)]
pub struct State {
    pub active_channel_key: Option<u32>,
    pub active_device: Option<Device>,
    pub active_tab: Tab,
    pub app_name: String,
    pub app_version: String,
    pub channels: HashMap<u32, Channel>,
    pub connection_attempt: u16,
    pub connection_state: ConnectionState,
    pub device_discovering_state: DeviceDiscoveringState,
    pub device_config: DeviceConfig,
    pub device_module_config: DeviceModuleConfig,
    pub discovered_devices: Vec<Device>,
    pub logs: Vec<LogRecord>,
    pub messages: HashMap<u32, VecDeque<Message>>,
    pub my_node_key: Option<u32>,
    pub need_clear_frame: bool,
    pub nodes_sort_by: NodesSortBy,
    pub nodes_sort: Vec<u32>,
    pub nodes: HashMap<u32, Node>,
    pub online_nodes: u16,
    pub reconnection_backoff: Option<Duration>,
    pub rx_t: Instant,
    pub rx: bool,
    pub settings_form_state: SettingsFormState,
    pub settings_form_data: Option<FormData>,
    pub splash_logo_t: Instant,
    pub splash_logo: bool,
    pub tcp_devices: Vec<HostAddr<String>>,
    pub toast_queue: VecDeque<Toast>,
    pub toast_t: Instant,
    pub toast: Option<Toast>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active_channel_key: None,
            active_device: None,
            active_tab: Tab::default(),
            app_name: crate::APP_NAME.to_owned(),
            app_version: crate::APP_VERSION.to_owned(),
            channels: HashMap::with_capacity(10),
            connection_attempt: 0,
            connection_state: ConnectionState::NotConnected,
            device_discovering_state: DeviceDiscoveringState::NotStarted,
            device_config: DeviceConfig::default(),
            device_module_config: DeviceModuleConfig::default(),
            discovered_devices: Vec::default(),
            logs: Vec::with_capacity(1000),
            messages: HashMap::default(),
            my_node_key: None,
            need_clear_frame: false,
            nodes_sort_by: NodesSortBy::Hops,
            nodes_sort: Vec::with_capacity(200),
            nodes: HashMap::with_capacity(200),
            online_nodes: 0,
            reconnection_backoff: None,
            rx_t: Instant::now(),
            rx: false,
            settings_form_state: SettingsFormState::Inactive,
            settings_form_data: None,
            splash_logo_t: Instant::now(),
            splash_logo: false,
            tcp_devices: Vec::default(),
            toast_queue: VecDeque::default(),
            toast_t: Instant::now(),
            toast: None,
        }
    }
}

impl State {
    pub fn get_my_node(&self) -> Option<&Node> {
        self.my_node_key.and_then(|key| self.nodes.get(&key))
    }

    pub fn get_active_channel(&self) -> Option<&Channel> {
        self.active_channel_key
            .and_then(|key| self.channels.get(&key))
    }
}
