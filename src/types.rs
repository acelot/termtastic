use crate::state::State;
use crate::ui::helpers::DateTimeExt;
use anyhow::anyhow;
use btleplug::api::BDAddr;
use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use emoji::Emoji;
use hostaddr::HostAddr;
use itertools::Itertools;
use meshtastic::protobufs::{channel, config, module_config, routing};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::sync::LazyLock;
use std::{
    collections::{BTreeSet, HashMap},
    fmt::Debug,
    time::Instant,
};
use strum::{Display, EnumCount, EnumIter, FromRepr};
use tracing::Level;

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub active_tab: Tab,
    #[serde(default)]
    pub active_device: Option<Device>,
    #[serde(default)]
    pub devices: BTreeSet<Device>,
    #[serde(default)]
    pub nodes_sort_by: NodesSortBy,
    #[serde(default)]
    pub nodes_filter: String,
    #[serde(default)]
    pub ui_config: UiConfig,
}

impl From<&State> for AppConfig {
    fn from(value: &State) -> Self {
        Self {
            active_tab: value.active_tab,
            active_device: value.active_device.clone(),
            devices: value.devices.clone(),
            nodes_sort_by: value.nodes_sort_by,
            nodes_filter: value.nodes_filter.clone(),
            ui_config: value.ui_config.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Default)]
pub struct UiConfig {
    #[serde(default)]
    pub is_top_padding_hidden: bool,
    #[serde(default)]
    pub is_bottom_padding_hidden: bool,
    #[serde(default)]
    pub is_left_padding_hidden: bool,
    #[serde(default)]
    pub is_right_padding_hidden: bool,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    ChatPurgeRequested(Chat),
    ChatSelected(Chat),
    ChatSwitchRequested,
    ChatMessageSubmitted {
        text: String,
        reply_message_id: Option<u32>,
    },
    ChatReactionSubmitted {
        emoji: &'static Emoji,
        reply_message_id: Option<u32>,
    },
    ConfigLoaded,
    CopyToClipboardRequested(String),
    DbLoadRequested(u32),
    DbLoadFinished,
    DeviceRebootRequested,
    DeviceRediscoverRequested,
    DeviceRemoveRequested(Device),
    DeviceSelected(Device),
    DeviceShutdownRequested,
    DeviceSubmitted(Device),
    DirectChatRequested(u32),
    DisconnectionRequested,
    InitializationRequested,
    NodeDeleteRequested(u32),
    NodeInfoBroadcastRequested,
    NodeInfoPopupCloseRequested,
    NodeInfoPopupOpenRequested(u32),
    NodesFilterChanged(String),
    NodesSortByNextRequested,
    NodesSortByPrevRequested,
    SettingsFormCancelRequested,
    SettingsFormItemSubmitted(&'static FormItem, FormValue),
    SettingsFormResetRequested,
    SettingsFormSaveRequested(FormId),
    SettingsFormSelected(FormId),
    SplashLogoRequested,
    TelemetryArrived(NodeTelemetry),
    TabNextRequested,
    TabPreviousRequested,
    TracerouteRequested(u32),
    TryingToQuit,
    QuitRequested,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Hash, Serialize, Deserialize)]
pub enum Device {
    Ble {
        name: Option<String>,
        address: BDAddr,
        id: Option<String>,
    },
    Tcp {
        name: Option<String>,
        address: HostAddr<String>,
    },
    Serial {
        name: Option<String>,
        address: String,
    },
}

impl Device {
    pub fn name(&self) -> Option<&str> {
        match self {
            Device::Ble { name, .. } => name.as_deref(),
            Device::Tcp { name, .. } => name.as_deref(),
            Device::Serial { name, .. } => name.as_deref(),
        }
    }

    pub fn with_name<S: Into<String>>(&self, name: S) -> Self {
        match self {
            Device::Ble { address, id, .. } => Device::Ble {
                name: Some(name.into()),
                address: *address,
                id: id.clone(),
            },
            Device::Tcp { address, .. } => Device::Tcp {
                name: Some(name.into()),
                address: address.clone(),
            },
            Device::Serial { address, .. } => Device::Serial {
                name: Some(name.into()),
                address: address.clone(),
            },
        }
    }

    pub fn without_name(&self) -> Self {
        match self {
            Device::Ble { address, id, .. } => Device::Ble {
                name: None,
                address: *address,
                id: id.clone(),
            },
            Device::Tcp { address, .. } => Device::Tcp {
                name: None,
                address: address.clone(),
            },
            Device::Serial { address, .. } => Device::Serial {
                name: None,
                address: address.clone(),
            },
        }
    }
}

impl Ord for Device {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Device::Tcp { .. }, Device::Ble { .. }) => std::cmp::Ordering::Less,
            (Device::Tcp { .. }, Device::Serial { .. }) => std::cmp::Ordering::Less,
            (
                Device::Tcp { address, .. },
                Device::Tcp {
                    address: other_address, ..
                },
            ) => address.cmp(other_address),

            (Device::Ble { .. }, Device::Tcp { .. }) => std::cmp::Ordering::Greater,
            (Device::Ble { .. }, Device::Serial { .. }) => std::cmp::Ordering::Less,
            (
                Device::Ble { address, id, .. },
                Device::Ble {
                    address: other_address,
                    id: other_id,
                    ..
                },
            ) => address.cmp(other_address).then(id.cmp(other_id)),

            (Device::Serial { .. }, Device::Tcp { .. }) => std::cmp::Ordering::Greater,
            (Device::Serial { .. }, Device::Ble { .. }) => std::cmp::Ordering::Greater,
            (
                Device::Serial { address, .. },
                Device::Serial {
                    address: other_address, ..
                },
            ) => address.cmp(other_address),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum DeviceDiscoveringState {
    #[default]
    NotStarted,
    Scanning,
    Finished,
}

impl DeviceDiscoveringState {
    pub fn is_scanning(&self) -> bool {
        self == &DeviceDiscoveringState::Scanning
    }
}

#[derive(Debug, Clone, Default)]
pub enum ConnectionState {
    #[default]
    NotConnected,
    ProblemDetected {
        since: Instant,
        error: String,
    },
    Connecting,
    LoadingConfig,
    Connected,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LogRecord {
    pub datetime: DateTime<Utc>,
    pub level: Level,
    pub source: String,
    pub message: String,
}

impl From<LogRecord> for String {
    fn from(val: LogRecord) -> Self {
        format!(
            "{} {} {}: {}",
            val.datetime.to_rfc3339(),
            val.level,
            &val.source,
            val.message
        )
    }
}

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Display, FromRepr, EnumIter, EnumCount, Serialize, Deserialize, Hash,
)]
pub enum Tab {
    #[default]
    #[strum(to_string = "chat")]
    Chat,
    #[strum(to_string = "nodes")]
    Nodes,
    #[strum(to_string = "settings")]
    Settings,
    #[strum(to_string = "connection")]
    Connection,
    #[strum(to_string = "logs")]
    Logs,
}

impl Tab {
    pub fn prev(self) -> Self {
        let current_index: usize = self as usize;
        let (previous_index, overflowed) = current_index.overflowing_sub(1);

        Self::from_repr(if overflowed { Tab::COUNT - 1 } else { previous_index }).unwrap_or(self)
    }

    pub fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);

        Self::from_repr(if next_index > Tab::COUNT - 1 { 0 } else { next_index }).unwrap_or(self)
    }
}

#[derive(Debug, Clone)]
pub struct Hotkey {
    pub key: String,
    pub label: String,
}

impl Hotkey {
    pub fn new<S: Into<String>>(key: S, label: S) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
        }
    }
}

#[derive(Debug, Clone)]
#[repr(u128)]
pub enum ToastKind {
    Success,
    Normal,
    Warning,
    Error,
}

impl ToastKind {
    pub fn timeout(&self) -> u128 {
        match self {
            Self::Success => 1500,
            Self::Normal => 1500,
            Self::Warning => 2000,
            Self::Error => 3000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub kind: ToastKind,
    pub text: String,
}

impl Toast {
    pub fn success<S: Into<String>>(text: S) -> Self {
        Self {
            kind: ToastKind::Success,
            text: text.into(),
        }
    }

    pub fn normal<S: Into<String>>(text: S) -> Self {
        Self {
            kind: ToastKind::Normal,
            text: text.into(),
        }
    }

    pub fn warning<S: Into<String>>(text: S) -> Self {
        Self {
            kind: ToastKind::Warning,
            text: text.into(),
        }
    }

    pub fn error<S: Into<String>>(text: S) -> Self {
        Self {
            kind: ToastKind::Error,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Display, FromRepr, EnumIter, EnumCount, Serialize, Deserialize, Hash, Default)]
pub enum NodesSortBy {
    #[default]
    #[strum(to_string = "hops / snr")]
    Hops,
    #[strum(to_string = "short name")]
    ShortName,
    #[strum(to_string = "long name")]
    LongName,
    #[strum(to_string = "last heard")]
    LastHeard,
    #[strum(to_string = "role / hops / snr")]
    Role,
    #[strum(to_string = "hardware / short name")]
    HwModel,
}

impl NodesSortBy {
    pub fn prev(self) -> Self {
        let current_index: usize = self as usize;
        let (previous_index, overflowed) = current_index.overflowing_sub(1);

        Self::from_repr(if overflowed {
            NodesSortBy::COUNT - 1
        } else {
            previous_index
        })
        .unwrap_or(self)
    }

    pub fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);

        Self::from_repr(if next_index > NodesSortBy::COUNT - 1 {
            0
        } else {
            next_index
        })
        .unwrap_or(self)
    }
}

pub trait HopsSnrRssiAware {
    fn hops(&self) -> Option<u32>;
    fn snr(&self) -> f32;
    fn rssi(&self) -> Option<i32>;
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeUser {
    pub id: String,
    pub short_name: String,
    pub long_name: String,
    pub role: i32,
    pub hw_model: i32,
    pub public_key: Vec<u8>,
    pub is_licensed: bool,
    pub is_unmessagable: Option<bool>,
}

impl From<&meshtastic::protobufs::User> for NodeUser {
    fn from(value: &meshtastic::protobufs::User) -> Self {
        Self {
            id: value.id.clone(),
            short_name: value.short_name.clone(),
            long_name: value.long_name.clone(),
            role: value.role,
            hw_model: value.hw_model,
            public_key: value.public_key.clone(),
            is_licensed: value.is_licensed,
            is_unmessagable: value.is_unmessagable,
        }
    }
}

impl From<NodeUser> for meshtastic::protobufs::User {
    fn from(val: NodeUser) -> Self {
        meshtastic::protobufs::User {
            id: val.id,
            long_name: val.long_name,
            short_name: val.short_name,
            #[allow(deprecated)]
            macaddr: Vec::new(),
            hw_model: val.hw_model,
            is_licensed: val.is_licensed,
            role: val.role,
            public_key: val.public_key,
            is_unmessagable: val.is_unmessagable,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub key: u32,
    pub user: Option<NodeUser>,
    pub hops: Option<u32>,
    pub last_heard: Option<DateTime<Utc>>,
    pub snr: f32,
    pub rssi: Option<i32>,
    pub is_favorite: bool,
    pub is_ignored: bool,
    pub is_muted: bool,
    pub fulltext: String,
}

pub static UNKNOWN_NODE: LazyLock<Node> = LazyLock::new(|| Node {
    key: 0,
    user: None,
    hops: None,
    last_heard: None,
    snr: 0.0,
    rssi: None,
    is_favorite: false,
    is_ignored: false,
    is_muted: false,
    fulltext: Default::default(),
});

impl Node {
    pub fn id(&self) -> String {
        format!("!{:x}", self.key)
    }

    pub fn short_name(&self) -> String {
        let id = self.id();

        self.user
            .as_ref().map(|u| u.short_name.clone())
            .unwrap_or_else(|| id[id.len().saturating_sub(4)..].to_string())
    }

    pub fn long_name(&self) -> String {
        self.user
            .as_ref().map(|u| u.long_name.clone())
            .unwrap_or(format!("Meshtastic {}", self.short_name()))
    }

    pub fn hw_model(&self) -> String {
        self.user
            .as_ref()
            .and_then(|u| meshtastic::protobufs::HardwareModel::try_from(u.hw_model).ok()).map(|hw| hw.as_str_name().to_owned())
            .unwrap_or("UNKNOWN".to_owned())
    }

    pub fn role(&self) -> String {
        self.user
            .as_ref()
            .and_then(|u| config::device_config::Role::try_from(u.role).ok()).map(|r| r.as_str_name().to_owned())
            .unwrap_or("UNKNOWN".to_owned())
    }

    pub fn update_fulltext(&mut self) {
        let is_direct = self.hops.map(|h| h == 0).unwrap_or(false);

        self.fulltext = [
            self.user
                .as_ref().map(|u| &u.short_name)
                .unwrap_or(&"?".to_owned())
                .to_lowercase(),
            self.user
                .as_ref().map(|u| &u.long_name)
                .unwrap_or(&"unknown".to_owned())
                .to_lowercase(),
            self.role().to_lowercase(),
            self.hw_model().to_lowercase(),
            self.key.to_string(),
            self.id(),
            if is_direct {
                "$direct".to_owned()
            } else {
                "$remote".to_owned()
            },
            if let Some(hops) = self.hops {
                format!("$hops{}", hops)
            } else {
                "".to_owned()
            },
            if self.is_favorite {
                "$favorite".to_owned()
            } else {
                "".to_owned()
            },
            if self.is_ignored {
                "$ignored".to_owned()
            } else {
                "".to_owned()
            },
            if self.is_muted {
                "$muted".to_owned()
            } else {
                "".to_owned()
            },
            if self.user.is_some() {
                "$stored".to_owned()
            } else {
                "$unknown".to_owned()
            },
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>()
        .join(" ");
    }
}

impl HopsSnrRssiAware for Node {
    fn hops(&self) -> Option<u32> {
        self.hops
    }

    fn snr(&self) -> f32 {
        self.snr
    }

    fn rssi(&self) -> Option<i32> {
        self.rssi
    }
}

impl TryFrom<&meshtastic::protobufs::NodeInfo> for Node {
    type Error = anyhow::Error;

    fn try_from(value: &meshtastic::protobufs::NodeInfo) -> Result<Self, Self::Error> {
        let mut node = Self {
            key: value.num,
            user: value.user.as_ref().map(|u| u.into()),
            hops: value.hops_away,
            last_heard: value.last_heard.timestamp_to_datetime(),
            snr: value.snr,
            rssi: None,
            is_favorite: value.is_favorite,
            is_ignored: value.is_ignored,
            is_muted: value.is_muted,
            fulltext: Default::default(),
        };

        node.update_fulltext();

        Ok(node)
    }
}

impl From<&meshtastic::protobufs::MeshPacket> for Node {
    fn from(packet: &meshtastic::protobufs::MeshPacket) -> Self {
        let mut node = Self {
            key: packet.from,
            user: None,
            hops: Some(packet.hop_start.saturating_sub(packet.hop_limit)),
            last_heard: packet.rx_time.timestamp_to_datetime(),
            snr: packet.rx_snr,
            rssi: Some(packet.rx_rssi),
            is_favorite: false,
            is_ignored: false,
            is_muted: false,
            fulltext: Default::default(),
        };

        node.update_fulltext();

        node
    }
}

impl TryFrom<(&meshtastic::protobufs::MeshPacket, &meshtastic::protobufs::User)> for Node {
    type Error = anyhow::Error;

    fn try_from(
        (packet, user): (&meshtastic::protobufs::MeshPacket, &meshtastic::protobufs::User),
    ) -> Result<Self, Self::Error> {
        let mut node = Self {
            key: packet.from,
            user: Some(user.into()),
            hops: Some(packet.hop_start.saturating_sub(packet.hop_limit)),
            last_heard: packet.rx_time.timestamp_to_datetime(),
            snr: packet.rx_snr,
            rssi: Some(packet.rx_rssi),
            is_favorite: false,
            is_ignored: false,
            is_muted: false,
            fulltext: Default::default(),
        };

        node.update_fulltext();

        Ok(node)
    }
}

impl TryInto<meshtastic::protobufs::User> for &Node {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<meshtastic::protobufs::User, Self::Error> {
        match &self.user {
            Some(user) => Ok(meshtastic::protobufs::User {
                id: self.id(),
                long_name: user.long_name.clone(),
                short_name: user.short_name.clone(),
                #[allow(deprecated)]
                macaddr: vec![],
                hw_model: user.hw_model,
                is_licensed: false,
                role: user.role,
                public_key: user.public_key.clone(),
                is_unmessagable: user.is_unmessagable,
            }),
            None => Err(anyhow!("no user information")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[repr(u32)]
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

impl From<channel::Role> for ChannelRole {
    fn from(value: channel::Role) -> Self {
        match value {
            channel::Role::Disabled => ChannelRole::Disabled,
            channel::Role::Primary => ChannelRole::Primary,
            channel::Role::Secondary => ChannelRole::Secondary,
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Hash)]
pub enum Chat {
    Channel(u32),
    Direct(u32),
}

impl Chat {
    pub fn is_direct(&self) -> bool {
        matches!(self, Chat::Direct(_))
    }
}

impl Ord for Chat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Chat::Channel(key), Chat::Channel(other_key)) => key.cmp(other_key),
            (Chat::Channel(_), Chat::Direct(_)) => std::cmp::Ordering::Less,
            (Chat::Direct(_), Chat::Channel(_)) => std::cmp::Ordering::Greater,
            (Chat::Direct(key), Chat::Direct(other_key)) => key.cmp(other_key),
        }
    }
}

impl From<(&Message, u32)> for Chat {
    fn from((message, my_node_key): (&Message, u32)) -> Self {
        if message.to == 0 || message.to == u32::MAX {
            Chat::Channel(message.channel)
        } else if my_node_key == message.to {
            Chat::Direct(message.from)
        } else {
            Chat::Direct(message.to)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub key: u32,
    pub id: u32,
    pub role: ChannelRole,
    pub name: String,
    pub psk: Vec<u8>,
    pub uplink_enabled: bool,
    pub downlink_enabled: bool,
    pub position_precision: u32,
    pub is_muted: bool,
    pub is_enabled: bool,
}

impl Channel {
    pub fn disabled(index: u32) -> Self {
        Self {
            key: index,
            id: 0,
            role: ChannelRole::Disabled,
            name: String::default(),
            psk: Vec::default(),
            uplink_enabled: false,
            downlink_enabled: false,
            position_precision: 0,
            is_muted: false,
            is_enabled: false,
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
                psk: settings.psk.clone(),
                uplink_enabled: settings.uplink_enabled,
                downlink_enabled: settings.downlink_enabled,
                position_precision: settings
                    .module_settings.map(|ms| ms.position_precision)
                    .unwrap_or(0),
                is_muted: settings
                    .module_settings.map(|ms| ms.is_muted)
                    .unwrap_or(false),
                is_enabled: value.role() != channel::Role::Disabled,
            },
            None => Channel::disabled(value.index as u32),
        }
    }
}

impl From<&Channel> for meshtastic::protobufs::Channel {
    fn from(val: &Channel) -> Self {
        let settings = val
            .is_enabled
            .then_some(Some(meshtastic::protobufs::ChannelSettings {
                name: val.name.clone(),
                psk: val.psk.clone(),
                #[allow(deprecated)]
                channel_num: val.key,
                id: val.key,
                uplink_enabled: val.uplink_enabled,
                downlink_enabled: val.downlink_enabled,
                module_settings: Some(meshtastic::protobufs::ModuleSettings {
                    position_precision: val.position_precision,
                    is_muted: val.is_muted,
                }),
            }))
            .unwrap_or(None);

        meshtastic::protobufs::Channel {
            index: val.key as i32,
            settings,
            role: match (val.key, val.is_enabled) {
                (0, true) => channel::Role::Primary as i32,
                (1..=u32::MAX, true) => channel::Role::Secondary as i32,
                (_, false) => channel::Role::Disabled as i32,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: u32,
    pub reply_message_id: u32,
    pub from: u32,
    pub to: u32,
    pub channel: u32,
    pub datetime: DateTime<Utc>,
    pub text: String,
    pub is_emoji: bool,
    pub reactions: Vec<u32>,
    pub hops: u32,
    pub snr: f32,
    pub rssi: i32,
    pub routing_error: Option<routing::Error>,
}

impl Message {
    pub fn text_oneline(&self) -> String {
        self.text.lines().join(" ")
    }
}

impl HopsSnrRssiAware for Message {
    fn hops(&self) -> Option<u32> {
        Some(self.hops)
    }

    fn snr(&self) -> f32 {
        self.snr
    }

    fn rssi(&self) -> Option<i32> {
        Some(self.rssi)
    }
}

impl TryFrom<(&meshtastic::protobufs::MeshPacket, &meshtastic::protobufs::Data)> for Message {
    type Error = anyhow::Error;

    fn try_from(
        (packet, data): (&meshtastic::protobufs::MeshPacket, &meshtastic::protobufs::Data),
    ) -> Result<Self, Self::Error> {
        if data.payload.is_empty() {
            return Err(anyhow!("payload is empty"));
        }

        Ok(Self {
            id: packet.id,
            reply_message_id: data.reply_id,
            from: packet.from,
            to: packet.to,
            channel: packet.channel,
            datetime: Utc
                .timestamp_opt(packet.rx_time as i64, 0)
                .single()
                .unwrap_or(Utc::now()),
            text: String::from_utf8(data.payload.clone())?,
            is_emoji: data.emoji > 0,
            reactions: Vec::default(),
            hops: packet.hop_start.saturating_sub(packet.hop_limit),
            snr: packet.rx_snr,
            rssi: packet.rx_rssi,
            routing_error: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Traceroute {
    pub message_id: u32,
    pub node_key: u32,
    pub datetime: DateTime<Utc>,
    pub acked: bool,
    pub routing_error: Option<routing::Error>,
    pub duration: TimeDelta,
    pub route_towards: Vec<TracerouteItem>,
    pub route_back: Vec<TracerouteItem>,
    pub state: TracerouteState,
}

impl Ord for Traceroute {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.datetime.cmp(&other.datetime).reverse()
    }
}

impl Eq for Traceroute {}

impl HopsSnrRssiAware for Vec<TracerouteItem> {
    fn hops(&self) -> Option<u32> {
        Some(self.iter().filter(|i| matches!(i, TracerouteItem::Node(_))).count() as u32)
    }

    fn snr(&self) -> f32 {
        if let Some(TracerouteItem::Snr(snr)) = self.first() {
            return *snr as f32;
        }

        0.0
    }

    fn rssi(&self) -> Option<i32> {
        None
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub enum TracerouteState {
    #[default]
    Started,
    RoutingError,
    TimedOut,
    Finished,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq)]
pub enum TracerouteItem {
    Snr(i32),
    Node(u32),
}

#[derive(Debug, Clone, Default)]
pub struct NodeLastTelemetry {
    pub device_metrics: Option<NodeLastTelemetryItem<meshtastic::protobufs::DeviceMetrics>>,
    pub environment_metrics: Option<NodeLastTelemetryItem<meshtastic::protobufs::EnvironmentMetrics>>,
    pub air_quality_metrics: Option<NodeLastTelemetryItem<meshtastic::protobufs::AirQualityMetrics>>,
    pub power_metrics: Option<NodeLastTelemetryItem<meshtastic::protobufs::PowerMetrics>>,
    pub local_stats: Option<NodeLastTelemetryItem<meshtastic::protobufs::LocalStats>>,
    pub health_metrics: Option<NodeLastTelemetryItem<meshtastic::protobufs::HealthMetrics>>,
    pub host_metrics: Option<NodeLastTelemetryItem<meshtastic::protobufs::HostMetrics>>,
    pub traffic_management_stats: Option<NodeLastTelemetryItem<meshtastic::protobufs::TrafficManagementStats>>,
}

#[derive(Debug, Clone)]
pub struct NodeLastTelemetryItem<T: Sized> {
    pub datetime: DateTime<Utc>,
    pub data: T,
}

#[derive(Debug, Clone)]
pub struct NodeTelemetry {
    pub node_key: u32,
    pub datetime: DateTime<Utc>,
    pub variant: meshtastic::protobufs::telemetry::Variant,
}

#[derive(Debug, Clone)]
pub enum TelemetryItem {
    Group {
        title: String,
        datetime: DateTime<Utc>,
        json: String,
    },
    Item {
        title: String,
        value: Option<String>,
        formatted_value: Option<String>,
    },
}

impl TelemetryItem {
    pub fn group(title: impl AsRef<str>, datetime: DateTime<Utc>, json: String) -> Self {
        Self::Group {
            title: title.as_ref().to_owned(),
            datetime,
            json,
        }
    }

    pub fn item(title: impl AsRef<str>, value: Option<impl ToString>, formatted_value: Option<impl ToString>) -> Self {
        Self::Item {
            title: title.as_ref().to_owned(),
            value: value.map(|v| v.to_string()),
            formatted_value: formatted_value.map(|v| v.to_string()),
        }
    }
}

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
pub enum FormId {
    RadioLora,
    RadioChannels,
    RadioSecurity,
    DeviceUser,
    DeviceDevice,
    DevicePosition,
    DevicePower,
    DeviceDisplay,
    DeviceBluetooth,
    DeviceNetwork,
    DeviceAdministration,
    ModuleMqtt,
    ModuleSerial,
    ModuleExternalNotification,
    ModuleStoreAndForward,
    ModuleRangeTest,
    ModuleTelemetry,
    ModuleCannedMessage,
    ModuleNeighborInfo,
    ModuleAmbientLighting,
    ModuleDetectionSensor,
    ModuleTrafficManagement,
    AppUi,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum SettingsFormState {
    #[default]
    Inactive,
    Loading {
        id: FormId,
    },
    LoadingFailed {
        id: FormId,
        error: String,
    },
    Loaded {
        id: FormId,
    },
    Saving {
        id: FormId,
    },
}

#[derive(Debug, Clone)]
pub enum SettingsItem {
    Group { title: &'static str },
    Form { title: &'static str, id: FormId },
}

impl SettingsItem {
    pub fn group(title: &'static str) -> Self {
        Self::Group { title }
    }

    pub fn form(title: &'static str, id: FormId) -> Self {
        Self::Form { title, id }
    }
}

pub type FormData = HashMap<String, FormValue>;

#[derive(Debug, Clone, PartialEq)]
pub enum FormValue {
    String(String),
    Int32(i32),
    UnsignedInt8(u8),
    UnsignedInt32(u32),
    UnsignedInt64(u64),
    Float32(f32),
    Bool(bool),
    Option(Option<Box<FormValue>>),
    Vec(Vec<FormValue>),
    Nested(FormData),
}

impl FormValue {
    pub fn as_string(&self) -> &String {
        let Self::String(v) = self else {
            panic!("expected String");
        };

        v
    }

    pub fn as_i32(&self) -> i32 {
        let Self::Int32(v) = self else {
            panic!("expected Int32");
        };

        *v
    }

    pub fn as_u8(&self) -> u8 {
        let Self::UnsignedInt8(v) = self else {
            panic!("expected UnsignedInt8");
        };

        *v
    }

    pub fn as_u32(&self) -> u32 {
        let Self::UnsignedInt32(v) = self else {
            panic!("expected UnsignedInt32");
        };

        *v
    }

    pub fn as_u64(&self) -> u64 {
        let Self::UnsignedInt64(v) = self else {
            panic!("expected UnsignedInt64");
        };

        *v
    }

    pub fn as_f32(&self) -> f32 {
        let Self::Float32(v) = self else {
            panic!("expected Float32");
        };

        *v
    }

    pub fn as_bool(&self) -> bool {
        let Self::Bool(v) = self else {
            panic!("expected Bool");
        };

        *v
    }

    pub fn as_option(&self) -> Option<&Self> {
        let Self::Option(v) = self else {
            panic!("expected Option");
        };

        v.as_deref()
    }

    pub fn as_option_mut(&mut self) -> Option<&mut Self> {
        let Self::Option(v) = self else {
            panic!("expected Option");
        };

        v.as_deref_mut()
    }

    pub fn as_vec(&self) -> &Vec<Self> {
        let Self::Vec(v) = self else {
            panic!("expected Vec");
        };

        v
    }

    pub fn as_nested(&self) -> &FormData {
        let Self::Nested(v) = self else {
            panic!("expected Nested");
        };

        v
    }

    pub fn as_nested_mut(&mut self) -> &mut FormData {
        let Self::Nested(v) = self else {
            panic!("expected Nested");
        };

        v
    }
}

impl std::fmt::Display for FormValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::String(v) => write!(f, "{}", v),
            Self::Int32(v) => write!(f, "{}", v),
            Self::UnsignedInt8(v) => write!(f, "{}", v),
            Self::UnsignedInt32(v) => write!(f, "{}", v),
            Self::UnsignedInt64(v) => write!(f, "{}", v),
            Self::Float32(v) => write!(f, "{}", v),
            Self::Bool(v) => {
                if *v {
                    write!(f, "true")
                } else {
                    write!(f, "false")
                }
            }
            Self::Option(v) => write!(f, "{:?}", v),
            Self::Vec(v) => write!(f, "{:?}", v),
            Self::Nested(v) => write!(f, "{:?}", v),
        }
    }
}

impl From<String> for FormValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<i32> for FormValue {
    fn from(value: i32) -> Self {
        Self::Int32(value)
    }
}

impl From<u8> for FormValue {
    fn from(value: u8) -> Self {
        Self::UnsignedInt8(value)
    }
}

impl From<u32> for FormValue {
    fn from(value: u32) -> Self {
        Self::UnsignedInt32(value)
    }
}

impl From<u64> for FormValue {
    fn from(value: u64) -> Self {
        Self::UnsignedInt64(value)
    }
}

impl From<f32> for FormValue {
    fn from(value: f32) -> Self {
        Self::Float32(value)
    }
}

impl From<bool> for FormValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<Vec<u8>> for FormValue {
    fn from(value: Vec<u8>) -> Self {
        Self::Vec(value.iter().map(|b| FormValue::UnsignedInt8(*b)).collect())
    }
}

#[derive(Debug, Clone)]
pub struct FormItem {
    pub key: FormItemKey,
    pub title: &'static str,
    pub description: Option<&'static str>,
    pub kind: FormItemKind,
    pub formatter: fn(&FormValue) -> String,
    pub validator: fn(&FormValue) -> anyhow::Result<()>,
}

impl FormItem {
    pub fn new(
        key: FormItemKey,
        title: &'static str,
        description: Option<&'static str>,
        kind: FormItemKind,
        formatter: fn(&FormValue) -> String,
        validator: fn(&FormValue) -> anyhow::Result<()>,
    ) -> Self {
        Self {
            key,
            title,
            description,
            kind,
            formatter,
            validator,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FormItemKey {
    Simple(&'static str),
    Custom {
        getter: fn(&FormData) -> FormValue,
        setter: fn(&mut FormData, FormValue),
    },
    None,
}

#[derive(Debug, Clone)]
pub enum FormItemKind {
    #[allow(unused)]
    ReadOnly,
    InputOfString,
    InputOfInt32,
    InputOfUnsignedInt32,
    InputOfFloat32,
    InputOfBase64,
    Enum(Vec<FormEnumVariant>),
    BitMask(Vec<FormBitMaskVariant>),
    Switch,
    Button(fn(&FormValue) -> FormValue),
    Action(AppEvent),
}

#[derive(Debug, Clone)]
pub struct FormEnumVariant {
    pub title: String,
    pub value: FormValue,
}

impl FormEnumVariant {
    pub fn new<S: Into<String>, F: Into<FormValue>>(title: S, value: F) -> Self {
        Self {
            title: title.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FormBitMaskVariant {
    pub title: String,
    pub value: u32,
}

impl FormBitMaskVariant {
    pub fn new<S: Into<String>>(title: S, value: u32) -> Self {
        Self {
            title: title.into(),
            value,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DeviceConfig {
    pub bluetooth: Option<config::BluetoothConfig>,
    pub device: Option<config::DeviceConfig>,
    pub device_ui: Option<meshtastic::protobufs::DeviceUiConfig>,
    pub display: Option<config::DisplayConfig>,
    pub lora: Option<config::LoRaConfig>,
    pub network: Option<config::NetworkConfig>,
    pub position: Option<config::PositionConfig>,
    pub power: Option<config::PowerConfig>,
    pub security: Option<config::SecurityConfig>,
    pub sessionkey: Option<config::SessionkeyConfig>,
}

#[derive(Debug, Clone, Default)]
pub struct DeviceModuleConfig {
    pub ambient_lighting: Option<module_config::AmbientLightingConfig>,
    pub audio: Option<module_config::AudioConfig>,
    pub canned_message: Option<module_config::CannedMessageConfig>,
    pub detection_sensor: Option<module_config::DetectionSensorConfig>,
    pub external_notification: Option<module_config::ExternalNotificationConfig>,
    pub mqtt: Option<module_config::MqttConfig>,
    pub neighbor: Option<module_config::NeighborInfoConfig>,
    pub paxcounter: Option<module_config::PaxcounterConfig>,
    pub range_test: Option<module_config::RangeTestConfig>,
    pub remote_hardware: Option<module_config::RemoteHardwareConfig>,
    pub serial: Option<module_config::SerialConfig>,
    pub status_message: Option<module_config::StatusMessageConfig>,
    pub store_forward: Option<module_config::StoreForwardConfig>,
    pub telemetry: Option<module_config::TelemetryConfig>,
    pub traffic_management: Option<module_config::TrafficManagementConfig>,
    pub tak: Option<module_config::TakConfig>,
}
