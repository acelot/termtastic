use std::{collections::HashMap, time::Instant};

use crate::{
    types::{
        AppConfig, Channel, ConnectionState, Device, DevicesConfig, DevicesDiscoveringState,
        LogRecord, Node,
    },
    ui::types::Tab,
};

#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub app_name: String,
    pub app_version: String,
    pub app_config: AppConfig,
    pub devices_config: DevicesConfig,
    pub connection_state: ConnectionState,
    pub active_tab: Tab,
    pub active_channel_id: Option<i32>,
    pub discovered_devices: Vec<Device>,
    pub device_discovering_state: DevicesDiscoveringState,
    pub nodes: HashMap<u32, Node>,
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
            app_config: AppConfig::default(),
            devices_config: DevicesConfig::default(),
            connection_state: ConnectionState::NotConnected,
            active_tab: Tab::Connection,
            active_channel_id: None,
            discovered_devices: Vec::default(),
            device_discovering_state: DevicesDiscoveringState::NeverStarted,
            nodes: HashMap::with_capacity(200),
            online_nodes: 0,
            channels: HashMap::with_capacity(10),
            logs: Vec::with_capacity(1000),
            rx_t: Instant::now(),
            rx: false,
        }
    }
}
