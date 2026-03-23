use hostaddr::HostAddr;

use crate::types::{
    AppConfig, Channel, ConnectionState, Device, DevicesConfig, DevicesDiscoveringState, LogRecord,
    Node,
};

#[derive(Debug, Clone)]
pub enum StateAction {
    SetAppConfig(AppConfig),
    SetAppConfigDevices(DevicesConfig),
    NextTab,
    PrevTab,
    SetSelectedDevice(Device),
    UnsetConnection,
    SetConnectionState(ConnectionState),
    AddLogRecord(LogRecord),
    SetDevicesDiscoveringState(DevicesDiscoveringState),
    SetDiscoveredDevices(Vec<Device>),
    AddTcpDevice(HostAddr<String>),
    RemoveTcpDevice(HostAddr<String>),
    AddNode(Node),
    SetChannel(i32, Channel),
    SetActiveChannel(i32),
    UnsetActiveChannel,
    SetOnlineNodes(u16),
    TriggerRx,
}
