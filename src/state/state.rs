use std::{collections::HashMap, time::Instant};

use chrono::Duration;
use hostaddr::HostAddr;

use crate::{
    types::{
        Channel, ConnectionState, Device, DevicesDiscoveringState, LogRecord, Node, NodesSortBy,
    },
    ui::types::Tab,
};

#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub app_name: String,
    pub app_version: String,
    pub connection_state: ConnectionState,
    pub connection_attempt: u16,
    pub connection_backoff: Duration,
    pub active_device: Option<Device>,
    pub active_tab: Tab,
    pub active_channel_id: Option<i32>,
    pub tcp_devices: Vec<HostAddr<String>>,
    pub discovered_devices: Vec<Device>,
    pub device_discovering_state: DevicesDiscoveringState,
    pub nodes: HashMap<u32, Node>,
    pub nodes_sort: Vec<u32>,
    pub nodes_sort_by: NodesSortBy,
    pub online_nodes: u16,
    pub channels: HashMap<i32, Channel>,
    pub logs: Vec<LogRecord>,
    pub rx_t: Instant,
    pub rx: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            app_name: crate::APP_NAME.to_owned(),
            app_version: crate::APP_VERSION.to_owned(),
            connection_state: ConnectionState::NotConnected,
            connection_attempt: 0,
            connection_backoff: Duration::zero(),
            active_device: None,
            active_tab: Tab::default(),
            active_channel_id: None,
            tcp_devices: Vec::default(),
            discovered_devices: Vec::default(),
            device_discovering_state: DevicesDiscoveringState::NeverStarted,
            nodes: HashMap::with_capacity(200),
            nodes_sort: Vec::with_capacity(200),
            nodes_sort_by: NodesSortBy::Hops,
            online_nodes: 0,
            channels: HashMap::with_capacity(10),
            logs: Vec::with_capacity(1000),
            rx_t: Instant::now(),
            rx: false,
        }
    }
}
