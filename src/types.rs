use std::{collections::HashMap, fmt::Debug, time::Instant};

use anyhow::anyhow;
use chrono::{DateTime, TimeZone, Utc};
use hostaddr::HostAddr;
use meshtastic::protobufs::{DeviceUiConfig, MeshPacket, User, config, module_config};
use ratatui::{
    style::{self, Stylize as _},
    text,
};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumCount, EnumIter, FromRepr};
use tokio::sync::watch::Ref;
use tracing::Level;

use crate::state::State;

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Default)]
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
    ChatMessageSubmitted {
        text: String,
        reply_message_id: Option<u32>,
    },
    SplashLogoRequested,
    DirectChatRequested(u32),
    SettingsFormSelected(FormId),
    SettingsFormLoadingCancelRequested,
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
pub enum DeviceDiscoveringState {
    NotStarted,
    Discovering,
    Failed(String),
    Done,
}

#[derive(Debug, Clone)]
pub enum ConnectionState {
    NotConnected,
    ProblemDetected { since: Instant, error: String },
    Connecting,
    Connected,
}

#[derive(Debug, Clone)]
pub struct LogRecord {
    pub datetime: DateTime<Utc>,
    pub level: Level,
    pub source: String,
    pub message: String,
}

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Display,
    FromRepr,
    EnumIter,
    EnumCount,
    Serialize,
    Deserialize,
    Hash,
)]
pub enum Tab {
    #[default]
    #[strum(to_string = "Chat")]
    Chat,
    #[strum(to_string = "Nodes")]
    Nodes,
    #[strum(to_string = "Settings")]
    Settings,
    #[strum(to_string = "Connection")]
    Connection,
    #[strum(to_string = "Logs")]
    Logs,
}

impl Tab {
    pub fn prev(self) -> Self {
        let current_index: usize = self as usize;
        let (previous_index, overflowed) = current_index.overflowing_sub(1);

        Self::from_repr(if overflowed {
            Tab::COUNT - 1
        } else {
            previous_index
        })
        .unwrap_or(self)
    }

    pub fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);

        Self::from_repr(if next_index > Tab::COUNT - 1 {
            0
        } else {
            next_index
        })
        .unwrap_or(self)
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
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

#[derive(Debug, Clone)]
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
            long_name: "Unknown".to_owned(),
            hops_away: None,
            last_heard: None,
            snr: 0.0,
            role: "UNKNOWN".to_owned(),
            hw_model: "UNKNOWN".to_owned(),
            my: false,
        }
    }

    pub fn to_span(&self) -> text::Span<'_> {
        text::Span::from(format!("{:^6}", self.short_name))
            .black()
            .patch_style(if self.my {
                style::Style::new().white().on_blue()
            } else {
                style::Style::new().on_green()
            })
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

impl TryFrom<(&MeshPacket, &User)> for Node {
    type Error = anyhow::Error;

    fn try_from((packet, user): (&MeshPacket, &User)) -> Result<Self, Self::Error> {
        let last_heard = DateTime::from_timestamp(packet.rx_time as i64, 0);

        Ok(Self {
            id: user.id.clone(),
            key: packet.from,
            short_name: user.short_name.clone(),
            long_name: user.long_name.clone(),
            hops_away: Some(packet.hop_start.saturating_sub(packet.hop_limit)),
            last_heard,
            snr: packet.rx_snr,
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct Message {
    pub id: u32,
    pub reply_message_id: u32,
    pub from: u32,
    pub datetime: DateTime<Utc>,
    pub text: String,
    pub reactions: HashMap<String, HashMap<u32, DateTime<Utc>>>,
    pub hops: Option<u32>,
    pub snr: f32,
    pub rssi: i32,
    pub acked: bool,
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
        Ok(Self {
            id: packet.id,
            reply_message_id: data.reply_id,
            from: packet.from,
            datetime: Utc
                .timestamp_opt(packet.rx_time as i64, 0)
                .single()
                .unwrap_or(Utc::now()),
            text: String::from_utf8(data.payload.clone())?,
            reactions: HashMap::default(),
            hops: Some(packet.hop_start.saturating_sub(packet.hop_limit)),
            snr: packet.rx_snr,
            rssi: packet.rx_rssi,
            acked: false,
        })
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
    ModuleMqtt,
    ModuleSerial,
    ModuleExternalNotification,
    ModuleStoreAndForward,
    ModuleRangeTest,
    ModuleTelemetry,
    ModuleCannedMessage,
    ModuleNeighborInfo,
    AppUi,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SettingsFormState {
    Inactive,
    Loading { id: FormId },
    LoadingFailed { id: FormId, error: String },
    Loaded { id: FormId },
    Saving { id: FormId },
    SavingFailed { id: FormId, error: String },
    Saved { id: FormId },
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

pub type FormData = HashMap<&'static str, FormValue>;

#[derive(Debug, Clone)]
pub enum FormValue {
    String(String),
    Int32(i32),
    UnsignedInt32(u32),
    Float32(f32),
    Bool(bool),
}

impl Into<String> for &FormValue {
    fn into(self) -> String {
        let FormValue::String(value) = self else {
            panic!()
        };
        value.clone()
    }
}

impl Into<i32> for &FormValue {
    fn into(self) -> i32 {
        let FormValue::Int32(value) = self else {
            panic!()
        };
        value.clone()
    }
}

impl Into<u32> for &FormValue {
    fn into(self) -> u32 {
        let FormValue::UnsignedInt32(value) = self else {
            panic!()
        };
        *value
    }
}

impl Into<f32> for &FormValue {
    fn into(self) -> f32 {
        let FormValue::Float32(value) = self else {
            panic!()
        };
        *value
    }
}

impl Into<bool> for &FormValue {
    fn into(self) -> bool {
        let FormValue::Bool(value) = self else {
            panic!()
        };
        *value
    }
}

#[derive(Debug, Clone)]
pub struct FormItem {
    pub key: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub kind: FormItemKind,
    pub formatter: fn(&FormValue) -> String,
}

impl FormItem {
    pub fn new(
        key: &'static str,
        title: &'static str,
        description: &'static str,
        kind: FormItemKind,
        formatter: fn(&FormValue) -> String,
    ) -> Self {
        Self {
            key,
            title,
            description,
            kind,
            formatter,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FormItemKind {
    InputOfString {
        maxlen: usize,
    },
    InputOfUInt32 {
        min: u32,
        max: u32,
    },
    InputOfFloat32 {
        min: f32,
        max: f32,
        precision: usize,
    },
    EnumOfString(Vec<FormEnumVariant<String>>),
    EnumOfInt32(Vec<FormEnumVariant<i32>>),
    EnumOfUnsignedInt32(Vec<FormEnumVariant<u32>>),
    EnumOfFloat32(Vec<FormEnumVariant<f32>>),
    BitMask(Vec<FormEnumVariant<u8>>),
    Switch,
    Button {
        event: AppEvent,
        confirm: bool,
    },
}

#[derive(Debug, Clone)]
pub struct FormEnumVariant<T>
where
    T: Sized,
{
    pub title: String,
    pub value: T,
}

impl<T> FormEnumVariant<T> {
    pub fn new<S: Into<String>>(title: S, value: T) -> Self {
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
    pub device_ui: Option<DeviceUiConfig>,
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
    pub map_report: Option<module_config::MapReportSettings>,
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
}
