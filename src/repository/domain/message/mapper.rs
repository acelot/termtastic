use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::repository::domain::message::Message;

impl From<crate::types::Message> for Message {
    fn from(value: crate::types::Message) -> Self {
        Self {
            id: value.id.as_bytes().to_vec(),
            reply_id: value.reply_id.as_bytes().to_vec(),
            packet_id: value.packet_id,
            reply_packet_id: value.reply_packet_id,
            from: value.from,
            to: value.to,
            channel: value.channel,
            datetime: value.datetime.to_rfc3339(),
            text: value.text,
            is_emoji: value.is_emoji,
            hops: value.hops,
            snr: value.snr,
            rssi: value.rssi,
            routing_error: value.routing_error.map(|e| e as i32),
        }
    }
}

impl TryFrom<Message> for crate::types::Message {
    type Error = anyhow::Error;

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::from_slice(&value.id)?,
            reply_id: Uuid::from_slice(&value.reply_id)?,
            packet_id: value.packet_id,
            reply_packet_id: value.reply_packet_id,
            from: value.from,
            to: value.to,
            channel: value.channel,
            datetime: DateTime::parse_from_rfc3339(&value.datetime)?.with_timezone(&Utc),
            text: value.text,
            is_emoji: value.is_emoji,
            reactions: Vec::default(),
            hops: value.hops,
            snr: value.snr,
            rssi: value.rssi,
            routing_error: value
                .routing_error
                .map(meshtastic::protobufs::routing::Error::try_from)
                .transpose()?,
        })
    }
}
