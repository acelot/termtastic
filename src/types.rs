use std::{collections::HashMap, time::Instant};

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use hostaddr::HostAddr;
use meshtastic::protobufs::PortNum;
use serde::{Deserialize, Serialize};
use tokio::sync::watch::Ref;
use tracing::Level;

use crate::{state::State, ui::types::Tab};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub active_tab: Tab,
    #[serde(default)]
    pub active_device: Option<Device>,
    #[serde(default)]
    pub tcp_devices: Vec<HostAddr<String>>,
    #[serde(default)]
    pub nodes_sort_by: NodesSortBy,
}

impl From<&Ref<'_, State>> for AppConfig {
    fn from(value: &Ref<'_, State>) -> Self {
        Self {
            active_tab: value.active_tab,
            active_device: value.active_device.clone(),
            tcp_devices: value.tcp_devices.clone(),
            nodes_sort_by: value.nodes_sort_by.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    ChannelSelected(u32),
    SwitchChannelRequested,
    DeviceRediscoverRequested,
    DeviceSelected(Device),
    DisconnectionRequested,
    InitializationRequested,
    NextTabRequested,
    PreviousTabRequested,
    TcpDeviceRemoved(HostAddr<String>),
    TcpDeviceSubmitted(HostAddr<String>),
    ChatMessageSubmitted(String),
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash)]
pub enum NodesSortBy {
    Hops,
    ShortName,
    LongName,
    LastHeard,
    Role,
    HwModel,
}

impl Default for NodesSortBy {
    fn default() -> Self {
        Self::Hops
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: String,
    pub key: u32,
    pub short_name: String,
    pub long_name: String,
    pub hops_away: Option<u32>,
    pub last_heard: Option<DateTime<Utc>>,
    pub snr: f32,
    pub role: String,
    pub hw_model: String,
    pub my: bool,
}

impl Node {
    pub fn unknown() -> Self {
        Self {
            id: "?".to_owned(),
            key: 0,
            short_name: "?".to_owned(),
            long_name: "unknown".to_owned(),
            hops_away: None,
            last_heard: None,
            snr: 0.0,
            role: "?".to_owned(),
            hw_model: "?".to_owned(),
            my: false,
        }
    }
}

impl TryFrom<&meshtastic::protobufs::NodeInfo> for Node {
    type Error = anyhow::Error;

    fn try_from(value: &meshtastic::protobufs::NodeInfo) -> Result<Self, Self::Error> {
        let user = value.user.as_ref().ok_or(anyhow!("no user information"))?;
        let last_heard = DateTime::from_timestamp(value.last_heard as i64, 0);

        Ok(Self {
            id: user.id.clone(),
            key: value.num,
            short_name: user.short_name.clone(),
            long_name: user.long_name.clone(),
            hops_away: value.hops_away,
            last_heard,
            snr: value.snr,
            role: user.role().as_str_name().to_string(),
            hw_model: user.hw_model().as_str_name().to_string(),
            my: false,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChannelRole {
    Disabled = 0,
    Primary = 1,
    Secondary = 2,
    Direct = 3,
}

impl ChannelRole {
    pub fn is_disabled(&self) -> bool {
        self == &Self::Disabled
    }

    pub fn is_direct(&self) -> bool {
        self == &Self::Direct
    }
}

impl From<meshtastic::protobufs::channel::Role> for ChannelRole {
    fn from(value: meshtastic::protobufs::channel::Role) -> Self {
        match value {
            meshtastic::protobufs::channel::Role::Disabled => ChannelRole::Disabled,
            meshtastic::protobufs::channel::Role::Primary => ChannelRole::Primary,
            meshtastic::protobufs::channel::Role::Secondary => ChannelRole::Secondary,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    pub key: u32,
    pub id: u32,
    pub role: ChannelRole,
    pub name: String,
}

impl Channel {
    pub fn disabled(index: u32) -> Self {
        Self {
            key: index,
            id: 0,
            role: ChannelRole::Disabled,
            name: String::default(),
        }
    }

    pub fn direct(node_key: u32) -> Self {
        Self {
            key: node_key,
            id: 0,
            role: ChannelRole::Direct,
            name: String::default(),
        }
    }
}

impl From<&meshtastic::protobufs::Channel> for Channel {
    fn from(value: &meshtastic::protobufs::Channel) -> Self {
        match &value.settings {
            Some(settings) => Self {
                key: value.index as u32,
                id: settings.id,
                role: value.role().into(),
                name: settings.name.to_string(),
            },
            None => Channel::disabled(value.index as u32),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub id: u32,
    pub reply_to: u32,
    pub from: u32,
    pub datetime: DateTime<Utc>,
    pub text: String,
    pub reactions: HashMap<String, HashMap<u32, DateTime<Utc>>>,
    pub hops: Option<u32>,
}

impl
    TryFrom<(
        &meshtastic::protobufs::MeshPacket,
        &meshtastic::protobufs::Data,
    )> for Message
{
    type Error = anyhow::Error;

    fn try_from(
        (packet, data): (
            &meshtastic::protobufs::MeshPacket,
            &meshtastic::protobufs::Data,
        ),
    ) -> Result<Self, Self::Error> {
        let text = match data.portnum() {
            PortNum::TextMessageApp | PortNum::ReplyApp => String::from_utf8(data.payload.clone())?,
            portnum => {
                return Err(anyhow::anyhow!(
                    "unsupported portnum: {}",
                    portnum.as_str_name()
                ));
            }
        };

        Ok(Self {
            id: packet.id,
            reply_to: data.reply_id,
            from: packet.from,
            datetime: Utc::now(),
            text,
            reactions: HashMap::default(),
            hops: Some(packet.hop_start - packet.hop_limit),
        })
    }
}
