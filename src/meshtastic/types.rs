use hostaddr::HostAddr;
use meshtastic::protobufs::from_radio::PayloadVariant;

#[derive(Debug, Clone)]
pub enum MeshtasticEvent {
    Connected,
    ConnectionError(String),
    RadioStopped,
    Disconnected,
    IncomingPacket(PayloadVariant),
    MessageAccepted,
    MessageRejected(String),
}

#[derive(Debug, Clone)]
pub enum CommandToMeshtastic {
    ConnectViaTcp(HostAddr<String>),
    ConnectViaBle(String),
    ConnectViaSerial(String),
    Disconnect,
    SendBroadcastTextMessage {
        my_node_id: u32,
        channel_id: u32,
        reply_message_id: Option<u32>,
        text: String,
    },
    SendDirectTextMessage {
        my_node_id: u32,
        node_id: u32,
        reply_message_id: Option<u32>,
        text: String,
    },
}
