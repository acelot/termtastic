use std::time::Instant;

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use hostaddr::HostAddr;
use meshtastic::protobufs::{
    Channel as MeshtasticChannel, NodeInfo as MeshtasticNodeInfo,
    channel::Role as MeshtasticChannelRole,
};
use serde::{Deserialize, Serialize};
use tracing::Level;
use tracing_unwrap::OptionExt;

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
    ChannelSelected(i32),
    SwitchChannelRequested,
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

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: String,
    pub number: u32,
    pub short_name: String,
    pub long_name: String,
    pub hops_away: Option<u32>,
    pub last_heard: Option<DateTime<Utc>>,
    pub snr: f32,
    pub role: String,
    pub hw_model: String,
}

impl TryFrom<&MeshtasticNodeInfo> for Node {
    type Error = anyhow::Error;

    fn try_from(value: &MeshtasticNodeInfo) -> Result<Self, Self::Error> {
        let user = value.user.as_ref().ok_or(anyhow!("no user information"))?;
        let last_heard = DateTime::from_timestamp(value.last_heard as i64, 0);

        Ok(Self {
            id: user.id.clone(),
            number: value.num,
            short_name: user.short_name.clone(),
            long_name: user.long_name.clone(),
            hops_away: value.hops_away,
            last_heard,
            snr: value.snr,
            role: user.role().as_str_name().to_string(),
            hw_model: user.hw_model().as_str_name().to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChannelRole {
    Disabled = 0,
    Primary = 1,
    Secondary = 2,
}

impl ChannelRole {
    pub fn is_disabled(&self) -> bool {
        self == &Self::Disabled
    }
}

impl From<MeshtasticChannelRole> for ChannelRole {
    fn from(value: MeshtasticChannelRole) -> Self {
        match value {
            MeshtasticChannelRole::Disabled => ChannelRole::Disabled,
            MeshtasticChannelRole::Primary => ChannelRole::Primary,
            MeshtasticChannelRole::Secondary => ChannelRole::Secondary,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    pub index: i32,
    pub id: u32,
    pub role: ChannelRole,
    pub name: String,
}

impl Channel {
    pub fn disabled(index: i32) -> Self {
        Self {
            index,
            id: 0,
            role: ChannelRole::Disabled,
            name: String::default(),
        }
    }
}

impl From<&MeshtasticChannel> for Channel {
    fn from(value: &MeshtasticChannel) -> Self {
        let settings = value.settings.as_ref().unwrap_or_log();

        Self {
            index: value.index,
            id: settings.id,
            role: value.role().into(),
            name: settings.name.to_string(),
        }
    }
}
