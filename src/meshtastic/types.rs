use hostaddr::HostAddr;

#[derive(Debug, Clone)]
pub enum MeshtasticEvent {
    Connected,
    ConnectionError(String),
    Disconnected,
    MessageArrived,
}

#[derive(Debug, Clone)]
pub enum MeshtasticCommand {
    ConnectViaTcp(HostAddr<String>),
    ConnectViaBle(String),
    ConnectViaSerial(String),
    Disconnect,
}
