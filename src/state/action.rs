use std::{collections::HashMap, time::Duration};

use chrono::{DateTime, Utc};
use meshtastic::protobufs::{config, module_config, routing};
use strum::EnumProperty;

use crate::types::{
    AppConfig, Channel, Chat, Device, FormData, FormId, FormItemKey, FormValue, LogRecord, Message, Node, NodePosition,
    NodeTelemetry, NodeUser, NodesSortBy, Toast, Traceroute, TracerouteItem, UiConfig,
};

#[derive(Debug, Clone, EnumProperty)]
pub enum StateAction {
    AppConfigApply(AppConfig),
    ActiveChatSet(Chat),
    ActiveChatUnset,
    ChannelSet(u32, Channel),
    ChatPurge(Chat),
    ConnectionFail(String),
    ConnectionStart,
    ConnectionStop,
    ConnectionLoadDbData,
    ConnectionLoadNodeConfig,
    ConnectionSuccess,
    ReconnectionBackoffSet(Duration),
    DbLoadFail,
    DbLoadSuccess,
    #[strum(props(rebroadcast = false))]
    DbIndicatorTrigger,
    DbNodesLoad(Vec<Node>),
    DbNodesLastTelemetryLoad(Vec<NodeTelemetry>),
    DbNodesLastPositionLoad(HashMap<u32, NodePosition>),
    DbMessagesLoad(Vec<Message>),
    DbTracerouteLoad(Vec<Traceroute>),
    DeviceActiveSet(Device),
    DevicesAdd(Device),
    DevicesRemove(Device),
    DevicesDiscoveredAdd(Device),
    DeviceDiscoveringStart,
    DeviceDiscoveringDone,
    DeviceConfigSet(config::PayloadVariant),
    DeviceModuleConfigSet(module_config::PayloadVariant),
    DeviceMetadataSet(meshtastic::protobufs::DeviceMetadata),
    DeviceCannedMessagesSet(String),
    #[strum(props(rebroadcast = false))]
    LogRecordAdd(LogRecord),
    DirectChatStart(u32),
    MessageAdd(Message),
    MessageErrorSet {
        message_id: u32,
        error: Option<routing::Error>,
        datetime: DateTime<Utc>,
    },
    MyNodeInfoSet {
        node_key: u32,
        nodes_count: u32,
    },
    #[strum(props(rebroadcast = false))]
    NodesLoaderIncrement,
    NodeInit(Node),
    NodeInfoKeySet(u32),
    NodeInfoKeyUnset,
    NodeInfoPositionSet(Vec<NodePosition>),
    NodeUpdate(Node),
    NodeUpdateLastHeard {
        node_key: u32,
        hops: u32,
        snr: f32,
        rssi: i32,
    },
    NodeDelete(u32),
    NodeOwnerSet(NodeUser),
    NodeLastTelemetrySet(NodeTelemetry),
    NodeLastPositionSet(NodePosition),
    NodesSortBySet(NodesSortBy),
    NodesFilterSet(String),
    NodesOnlineSet(u32),
    #[strum(props(rebroadcast = false))]
    RxIndicatorTrigger,
    #[strum(props(rebroadcast = false))]
    SplashLogo,
    TabSwitchToNext,
    TabSwitchToPrevious,
    #[strum(props(rebroadcast = false))]
    Toast(Toast),
    TracerouteStart {
        message_id: u32,
        node_key: u32,
        datetime: DateTime<Utc>,
    },
    TracerouteTimeout {
        message_id: u32,
    },
    TracerouteFinish {
        message_id: u32,
        datetime: DateTime<Utc>,
        route_towards: Vec<TracerouteItem>,
        route_back: Vec<TracerouteItem>,
    },
    TracerouteRemove {
        message_id: u32,
    },
    SettingsFormLoadingStart {
        id: FormId,
    },
    SettingsFormLoadingFail {
        id: FormId,
        error: String,
    },
    SettingsFormLoadingDone {
        id: FormId,
        data: FormData,
    },
    SettingsFormSavingStart {
        id: FormId,
    },
    SettingsFormSavingFailed {
        id: FormId,
    },
    SettingsFormClose,
    SettingsFormReset,
    SettingsFormValueSet {
        key: FormItemKey,
        value: FormValue,
    },
    UiConfigSet {
        config: UiConfig,
    },
}

impl StateAction {
    pub fn should_rebroadcast(&self) -> bool {
        self.get_bool("rebroadcast").unwrap_or(true)
    }
}
