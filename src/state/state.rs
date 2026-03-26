use std::{
    collections::{HashMap, VecDeque},
    time::Instant,
};

use chrono::Duration;
use hostaddr::HostAddr;

use crate::{
    types::{
        Channel, ConnectionState, Device, DevicesDiscoveringState, LogRecord, Message, Node,
        NodesSortBy,
    },
    ui::types::Tab,
};

#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub active_channel_key: Option<u32>,
    pub active_device: Option<Device>,
    pub active_tab: Tab,
    pub app_name: String,
    pub app_version: String,
    pub channels: HashMap<u32, Channel>,
    pub connection_attempt: u16,
    pub connection_backoff: Duration,
    pub connection_state: ConnectionState,
    pub device_discovering_state: DevicesDiscoveringState,
    pub discovered_devices: Vec<Device>,
    pub logs: Vec<LogRecord>,
    pub my_node_number: Option<u32>,
    pub messages: HashMap<u32, VecDeque<Message>>,
    pub nodes_sort_by: NodesSortBy,
    pub nodes_sort: Vec<u32>,
    pub nodes: HashMap<u32, Node>,
    pub online_nodes: u16,
    pub rx_t: Instant,
    pub rx: bool,
    pub tcp_devices: Vec<HostAddr<String>>,
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
            connection_backoff: Duration::zero(),
            connection_state: ConnectionState::NotConnected,
            device_discovering_state: DevicesDiscoveringState::NeverStarted,
            discovered_devices: Vec::default(),
            logs: Vec::with_capacity(1000),
            my_node_number: None,
            messages: HashMap::default(),
            nodes_sort_by: NodesSortBy::Hops,
            nodes_sort: Vec::with_capacity(200),
            nodes: HashMap::with_capacity(200),
            online_nodes: 0,
            rx_t: Instant::now(),
            rx: false,
            tcp_devices: Vec::default(),
        }
    }
}
