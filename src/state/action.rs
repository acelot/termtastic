use hostaddr::HostAddr;

use crate::types::{AppConfig, Channel, Device, LogRecord, Message, Node, NodesSortBy, Toast};

#[derive(Debug, Clone)]
pub enum StateAction {
    AppConfigApply(AppConfig),
    ChannelActiveSet(u32),
    ChannelActiveUnset,
    ChannelEnsure(u32, Channel),
    ConnectionFail(String),
    ConnectionStart,
    ConnectionStop,
    ConnectionSuccess,
    DeviceActiveSet(Device),
    DevicesAddTcp(HostAddr<String>),
    DiscoveredDevicesSet(Vec<Device>),
    DevicesRemoveTcp(HostAddr<String>),
    LogRecordAdd(LogRecord),
    MessageAdd(u32, Message),
    MessageReactionAdd {
        channel_key: u32,
        message_id: u32,
        emoji: String,
        node_key: u32,
    },
    MyNodeKeySet(u32),
    NodeAdd(Node),
    NodeUpdateLastHeard(u32),
    NodeSetSnr(u32, f32),
    NodesSortBySet(NodesSortBy),
    OnlineNodesSet(u16),
    RxTrigger,
    TabSwitchToNext,
    TabSwitchToPrevious,
    FrameCleared,
    Toast(Toast),
}
