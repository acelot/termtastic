use std::time::Instant;

use chrono::{DateTime, Utc};
use circular_queue::CircularQueue;
use hostaddr::HostAddr;
use serde::{Deserialize, Serialize};
use tracing::Level;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Default)]
pub struct AppConfig {
    pub selected_device: Option<Device>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Default)]
pub struct DevicesConfig {
    pub tcp_devices: Vec<HostAddr<String>>,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    ChatConversationSelected(String),
    ChatMessageSubmitted(String),
    DeviceRediscoverRequested,
    DeviceSelected(Device),
    DisconnectionRequested,
    InitializationRequested,
    NextTabRequested,
    PreviousTabRequested,
    TcpDeviceRemoved(HostAddr<String>),
    TcpDeviceSubmitted(HostAddr<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatMessage {
    pub node_id: String,
    pub node_name: String,
    pub datetime: DateTime<Utc>,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatConversation {
    pub name: String,
    pub messages: CircularQueue<ChatMessage>,
    pub offset: u64,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Serialize, Deserialize, Hash)]
pub enum Device {
    Ble { name: String, address: String },
    Tcp(HostAddr<String>),
    Serial(String),
}

impl Ord for Device {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Device::Tcp { .. }, Device::Ble { .. }) => std::cmp::Ordering::Less,
            (Device::Tcp { .. }, Device::Serial { .. }) => std::cmp::Ordering::Less,
            (Device::Tcp(hostaddr), Device::Tcp(other_hostaddr)) => hostaddr.cmp(other_hostaddr),

            (Device::Ble { .. }, Device::Tcp { .. }) => std::cmp::Ordering::Greater,
            (Device::Ble { .. }, Device::Serial { .. }) => std::cmp::Ordering::Less,
            (
                Device::Ble { address, .. },
                Device::Ble {
                    address: other_address,
                    ..
                },
            ) => address.cmp(other_address),

            (Device::Serial { .. }, Device::Tcp { .. }) => std::cmp::Ordering::Greater,
            (Device::Serial { .. }, Device::Ble { .. }) => std::cmp::Ordering::Greater,
            (Device::Serial(address), Device::Serial(other_address)) => address.cmp(other_address),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    NotConnected,
    ProblemDetected { since: Instant, error: String },
    Connecting,
    Connected,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LogRecord {
    pub datetime: DateTime<Utc>,
    pub level: Level,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DevicesDiscoveringState {
    NeverStarted,
    InProgress,
    Error(String),
    Finished,
}
