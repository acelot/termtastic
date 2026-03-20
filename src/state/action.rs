use hostaddr::HostAddr;

use crate::types::{
    AppConfig, ConnectionState, Device, DevicesConfig, DevicesDiscoveringState, LogRecord,
};

#[derive(Debug, Clone)]
pub enum StateAction {
    SetAppConfig(AppConfig),
    SetAppConfigDevices(DevicesConfig),
    NextTab,
    PrevTab,
    SetSelectedConnection(Device),
    UnsetConnection,
    SetConnectionState(ConnectionState),
    PushLogRecord(LogRecord),
    SetDevicesDiscoveringState(DevicesDiscoveringState),
    SetDiscoveredDevices(Vec<Device>),
    AddTcpDevice(HostAddr<String>),
    RemoveTcpDevice(HostAddr<String>),
}
