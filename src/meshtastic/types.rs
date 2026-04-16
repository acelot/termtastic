use hostaddr::HostAddr;
use meshtastic::protobufs::{config, from_radio, module_config};

#[derive(Debug, Clone)]
pub enum MeshtasticEvent {
    Connected,
    ConnectionError(String),
    Disconnected,
    IncomingConfig(config::PayloadVariant),
    IncomingModuleConfig(module_config::PayloadVariant),
    IncomingPacket(from_radio::PayloadVariant),
    MessageAccepted,
    #[allow(dead_code)]
    MessageRejected(String),
    RadioStopped,
    ConfigSaveError(String),
    ConfigSaved,
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
    SaveConfig {
        my_node_id: u32,
        config: config::PayloadVariant,
    },
}
