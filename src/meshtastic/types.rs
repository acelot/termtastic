use hostaddr::HostAddr;
use meshtastic::protobufs::from_radio::PayloadVariant;

#[derive(Debug, Clone)]
pub enum MeshtasticEvent {
    Connected,
    ConnectionError(String),
    RadioStopped,
    Disconnected,
    IncomingPacket(PayloadVariant),
}

#[derive(Debug, Clone)]
pub enum MeshtasticCommand {
    ConnectViaTcp(HostAddr<String>),
    ConnectViaBle(String),
    ConnectViaSerial(String),
    Disconnect,
}
