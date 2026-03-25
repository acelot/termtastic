use hostaddr::HostAddr;

use crate::types::{AppConfig, Channel, Device, LogRecord, Node, NodesSortBy};

#[derive(Debug, Clone)]
pub enum StateAction {
    AppConfigApply(AppConfig),
    ChannelActiveSet(i32),
    ChannelActiveUnset,
    ChannelAdd(i32, Channel),
    ConnectionFail(String),
    ConnectionStart,
    ConnectionStop,
    ConnectionSuccess,
    DeviceActiveSet(Device),
    DevicesAddTcp(HostAddr<String>),
    DevicesDiscoveringFail(String),
    DevicesDiscoveringStart,
    DevicesDiscoveringSuccess(Vec<Device>),
    DevicesRemoveTcp(HostAddr<String>),
    LogRecordAdd(LogRecord),
    NodeAdd(Node),
    NodeSetLastHeard(u32),
    NodeSetSnr(u32, f32),
    NodesSortBySet(NodesSortBy),
    OnlineNodesSet(u16),
    RxTrigger,
    TabSwitchToNext,
    TabSwitchToPrevious,
}
