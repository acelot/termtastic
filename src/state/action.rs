use std::{collections::HashMap, time::Duration};

use jiff::Timestamp;
use meshtastic::protobufs::{DeviceMetrics, config, module_config, routing};
use strum::EnumProperty;
use uuid::Uuid;

use crate::types::{
    AppConfig, AppGeneralConfig, AppUiConfig, Channel, Chat, ChatPosition, Device, FormData, FormId, FormItemKey,
    FormValue, LogRecord, Message, Node, NodePosition, NodeTelemetry, NodeTelemetryTyped, NodeUser, NodesSortBy, Toast,
    Traceroute, TracerouteItem,
};

#[derive(Debug, Clone, EnumProperty)]
pub enum StateAction {
    AppConfigApply(AppConfig),
    ActiveChatSet(Chat),
    ActiveChatUnset,
    ChannelSet {
        key: u32,
        channel: Channel,
    },
    ChatPurge(Chat),
    ChatSelectMessage(Chat, Uuid),
    ConnectionFail {
        error: String,
    },
    ConnectionStart,
    ConnectionStop,
    ConnectionLoadDbData,
    ConnectionLoadNodeConfig,
    ConnectionSuccess,
    ReconnectionBackoffSet(Duration),
    DbLoadRollback,
    DbLoadCommit,
    #[strum(props(rebroadcast = false))]
    DbIndicatorTrigger,
    #[strum(props(rebroadcast = false))]
    DbNodesLoad(Vec<Node>),
    #[strum(props(rebroadcast = false))]
    DbNodesLastTelemetryLoad(Vec<NodeTelemetry>),
    #[strum(props(rebroadcast = false))]
    DbNodesLastPositionLoad(HashMap<u32, NodePosition>),
    #[strum(props(rebroadcast = false))]
    DbChatPositionLoad(HashMap<Chat, ChatPosition>),
    #[strum(props(rebroadcast = false))]
    DbMessagesLoad(Vec<Message>),
    #[strum(props(rebroadcast = false))]
    DbTracerouteLoad(Vec<Traceroute>),
    #[strum(props(rebroadcast = false))]
    DbMyNodeDeviceMetricsLoad(Vec<NodeTelemetryTyped<DeviceMetrics>>),
    DeviceActiveSet(Device),
    DevicesAdd(Device),
    DevicesRemove(Device),
    DevicesDiscoveredAdd(Device),
    DeviceDiscoveringStart,
    DeviceDiscoveringDone,
    DeviceConfigSet(config::PayloadVariant),
    DeviceModuleConfigSet(module_config::PayloadVariant),
    DeviceMetadataSet(meshtastic::protobufs::DeviceMetadata),
    DeviceCannedMessagesSet {
        messages: String,
    },
    #[strum(props(rebroadcast = false))]
    LogRecordAdd(LogRecord),
    DirectChatStart {
        node_key: u32,
    },
    MessageAdd(Message),
    MessageSending {
        packet_id: u32,
    },
    MessageDeliveredToRecipient {
        packet_id: u32,
    },
    #[allow(dead_code)]
    MessageDeliveredToRecipientProven {
        packet_id: u32,
    },
    #[allow(dead_code)]
    MessageDeliveryProofFailed {
        packet_id: u32,
    },
    MessageDeliveredToMesh {
        packet_id: u32,
    },
    MessageRoutingError {
        packet_id: u32,
        error: routing::Error,
        datetime: Timestamp,
    },
    PreviousMessagesLoadPending(Chat),
    PreviousMessagesLoadFailed,
    PreviousMessagesLoadFinished {
        chat: Chat,
        messages: Vec<Message>,
    },
    MyNodeInfoSet {
        node_key: u32,
        nodes_count: u32,
    },
    #[strum(props(rebroadcast = false))]
    NodesLoaderIncrement,
    NodeInit(Node),
    NodeInfoKeySet {
        node_key: u32,
    },
    NodeInfoKeyUnset,
    NodeInfoPositionSet(Vec<NodePosition>),
    NodeUpdate(Node),
    NodeUpdateLastHeard {
        node_key: u32,
        hops: u32,
        snr: f32,
        rssi: Option<i32>,
    },
    NodeDelete {
        node_key: u32,
    },
    NodeOwnerSet(NodeUser),
    NodeLastTelemetrySet(NodeTelemetry),
    NodeLastPositionSet(NodePosition),
    NodesSortBySet(NodesSortBy),
    NodesFilterSet {
        query: String,
    },
    NodesOnlineSet {
        count: u32,
    },
    #[strum(props(rebroadcast = false))]
    RxIndicatorTrigger,
    #[strum(props(rebroadcast = false))]
    SplashLogo,
    TabSwitchToNext,
    TabSwitchToPrevious,
    #[strum(props(rebroadcast = false))]
    Toast(Toast),
    #[strum(props(rebroadcast = false))]
    UpdateAvailableVersionSet(Option<String>),
    #[strum(props(rebroadcast = false))]
    UpdateCheckTimestampSet(Timestamp),
    TracerouteStart {
        message_id: u32,
        node_key: u32,
        datetime: Timestamp,
    },
    TracerouteTimeout {
        message_id: u32,
    },
    TracerouteFinish {
        message_id: u32,
        datetime: Timestamp,
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
    AppUiConfigSet(AppUiConfig),
    AppGeneralConfigSet(AppGeneralConfig),
}

impl StateAction {
    pub fn should_rebroadcast(&self) -> bool {
        self.get_bool("rebroadcast").unwrap_or(true)
    }
}
