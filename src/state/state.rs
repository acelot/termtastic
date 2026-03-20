use std::collections::HashMap;

use circular_queue::CircularQueue;

use crate::{
    types::{
        AppConfig, ChatConversation, ConnectionState, Device, DevicesConfig,
        DevicesDiscoveringState, LogRecord,
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
    pub active_chat: Option<String>,
    pub conversations: HashMap<String, ChatConversation>,
    pub discovered_devices: Vec<Device>,
    pub device_discovering_state: DevicesDiscoveringState,
    pub logs: Vec<LogRecord>,
}

impl Default for State {
    fn default() -> Self {
        let mut conversations = HashMap::new();
        conversations.insert(
            "some".to_string(),
            ChatConversation {
                name: "test".to_owned(),
                messages: CircularQueue::with_capacity(100),
                offset: 0,
            },
        );

        Self {
            app_name: crate::APP_NAME.to_owned(),
            app_version: crate::APP_VERSION.to_owned(),
            app_config: AppConfig::default(),
            devices_config: DevicesConfig::default(),
            connection_state: ConnectionState::NotConnected,
            active_tab: Tab::Connection,
            active_chat: Some("some".to_string()),
            conversations,
            discovered_devices: Vec::default(),
            device_discovering_state: DevicesDiscoveringState::NeverStarted,
            logs: Vec::default(),
        }
    }
}
