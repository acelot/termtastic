use crate::repository::domain::message::Message;

impl From<crate::types::Message> for Message {
    fn from(value: crate::types::Message) -> Self {
        Self {
            id: value.id,
            reply_message_id: value.reply_message_id,
            from: value.from,
            to: value.to,
            channel: value.channel,
            datetime: value.datetime,
            text: value.text,
            is_emoji: value.is_emoji,
            hops: value.hops,
            snr: value.snr,
            rssi: value.rssi,
            routing_error: value.routing_error.map(|e| e as i32),
        }
    }
}

impl From<Message> for crate::types::Message {
    fn from(value: Message) -> Self {
        Self {
            id: value.id,
            reply_message_id: value.reply_message_id,
            from: value.from,
            to: value.to,
            channel: value.channel,
            datetime: value.datetime,
            text: value.text,
            is_emoji: value.is_emoji,
            reactions: Vec::default(),
            hops: value.hops,
            snr: value.snr,
            rssi: value.rssi,
            routing_error: value
                .routing_error
                .map(meshtastic::protobufs::routing::Error::try_from)
                .transpose()
                .unwrap_or(None),
        }
    }
}
