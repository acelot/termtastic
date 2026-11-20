use jiff::Timestamp;
use meshtastic::protobufs::routing;
use rusqlite::{ToSql, types::ToSqlOutput};
use uuid::Uuid;

use crate::{repository::domain::message::Message, types::DeliveryState};

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
            datetime: value.datetime.to_string(),
            text: value.text,
            is_emoji: value.is_emoji,
            hops: value.hops,
            snr: value.snr,
            rssi: value.rssi,
            delivery_state: value.delivery_state.map(Into::into),
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
            datetime: value.datetime.parse::<Timestamp>()?,
            text: value.text,
            is_emoji: value.is_emoji,
            reactions: Vec::default(),
            hops: value.hops,
            snr: value.snr,
            rssi: value.rssi,
            delivery_state: value.delivery_state.map(DeliveryState::try_from).transpose()?,
        })
    }
}

impl TryFrom<i32> for DeliveryState {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Sending),
            1 => Ok(Self::DeliveredToRecipient),
            2 => Ok(Self::DeliveredToRecipientProven),
            3 => Ok(Self::DeliveryProofFailed),
            4 => Ok(Self::DeliveredToMesh),
            32.. => Ok(Self::RoutingError(routing::Error::try_from(value - 32)?)),
            _ => Err(anyhow::anyhow!("invalid DeliveryState")),
        }
    }
}

impl From<DeliveryState> for i32 {
    fn from(val: DeliveryState) -> Self {
        match val {
            DeliveryState::Sending => 0,
            DeliveryState::DeliveredToRecipient => 1,
            DeliveryState::DeliveredToRecipientProven => 2,
            DeliveryState::DeliveryProofFailed => 3,
            DeliveryState::DeliveredToMesh => 4,
            DeliveryState::RoutingError(e) => e as i32 + 32,
        }
    }
}

impl ToSql for DeliveryState {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let value: i32 = Into::into(self.clone());
        Ok(ToSqlOutput::Owned(rusqlite::types::Value::Integer(value as i64)))
    }
}
