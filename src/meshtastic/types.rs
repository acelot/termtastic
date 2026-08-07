use crate::types::FormId;
use btleplug::api::BDAddr;
use emoji::Emoji;
use hostaddr::HostAddr;
use meshtastic::protobufs::{config, from_radio, module_config};

#[derive(Debug, Clone)]
pub enum MeshtasticEvent {
    Connected,
    ConnectionError(String),
    Disconnected,
    IncomingPacket(from_radio::PayloadVariant),
    MessageAccepted,
    MessageRejected(String),
    RadioStopped,
    ConfigSaved(FormId),
    ConfigSaveFailed(FormId),
    ChannelsSaved(FormId),
    ChannelsSaveFailed(FormId),
    UserSaved(FormId),
    UserSaveFailed(FormId),
    NodeInfoBroadcastSent,
    NodeInfoBroadcastFailed(String),
    NodeRemoveAccepted,
    NodeRemoveFailed(String),
    TracerouteStarted,
    TracerouteFailed(String),
}

#[derive(Debug, Clone)]
pub enum TextMessage {
    Text(String),
    Emoji(&'static Emoji),
}

#[derive(Debug, Clone)]
pub enum CommandToMeshtastic {
    ConnectViaTcp(HostAddr<String>),
    ConnectViaBle(BDAddr, Option<String>),
    ConnectViaSerial(String),
    Disconnect,
    Reboot {
        secs: i32,
        my_node_num: u32,
    },
    Shutdown {
        secs: i32,
        my_node_num: u32,
    },
    SendBroadcastTextMessage {
        channel_id: u32,
        reply_message_id: Option<u32>,
        text: TextMessage,
        my_node_num: u32,
    },
    SendDirectTextMessage {
        node_num: u32,
        reply_message_id: Option<u32>,
        text: TextMessage,
        my_node_num: u32,
    },
    BroadcastNodeInfo {
        channel_id: u32,
        user: meshtastic::protobufs::User,
    },
    SaveConfig {
        form_id: FormId,
        config: config::PayloadVariant,
        my_node_num: u32,
    },
    SaveModuleConfig {
        form_id: FormId,
        config: module_config::PayloadVariant,
        my_node_num: u32,
    },
    SaveChannelsConfig {
        form_id: FormId,
        channels: Vec<meshtastic::protobufs::Channel>,
        my_node_num: u32,
    },
    SaveUser {
        form_id: FormId,
        user: meshtastic::protobufs::User,
        my_node_num: u32,
    },
    DeleteNode {
        node_num: u32,
        my_node_num: u32,
    },
    LoadCannedMessages {
        my_node_num: u32,
    },
    SaveCannedMessages {
        messages: String,
        my_node_num: u32,
    },
    RunTraceroute {
        node_num: u32,
        my_node_num: u32,
    },
}
