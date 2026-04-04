use std::time::Duration;

use hostaddr::HostAddr;

use crate::types::{AppConfig, Channel, Device, LogRecord, Message, Node, NodesSortBy, Tab, Toast};

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
    ReconnectionBackoffSet(Duration),
    DeviceActiveSet(Device),
    DevicesAddTcp(HostAddr<String>),
    DeviceDiscoveringStart,
    DeviceDiscoveringFail(String),
    DeviceDiscoveringDone(Vec<Device>),
    DevicesRemoveTcp(HostAddr<String>),
    LogRecordAdd(LogRecord),
    DirectChatStart(u32),
    MessageAdd(u32, Message),
    MessageReactionAdd {
        channel_key: u32,
        message_id: u32,
        emoji: String,
        node_key: u32,
    },
    MessageAck(u32, u32),
    MyNodeKeySet(u32),
    NodeAdd(Node),
    NodeUpdateLastHeard {
        node_key: u32,
        hops: u32,
        snr: f32,
    },
    NodesSortBySet(NodesSortBy),
    RxTrigger,
    SplashLogo,
    TabSwitchTo(Tab),
    TabSwitchToNext,
    TabSwitchToPrevious,
    FrameCleared,
    Toast(Toast),
}
