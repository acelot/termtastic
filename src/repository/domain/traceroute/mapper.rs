use crate::{repository::domain::traceroute::Traceroute, types::TracerouteState};
use chrono::TimeDelta;

impl TryFrom<crate::types::Traceroute> for Traceroute {
    type Error = anyhow::Error;

    fn try_from(value: crate::types::Traceroute) -> Result<Self, Self::Error> {
        Ok(Self {
            message_id: value.message_id,
            node_key: value.node_key,
            datetime: value.datetime,
            routing_error: value.routing_error.map(|e| e as i32),
            duration_ms: value.duration.num_milliseconds(),
            route_towards: serde_json::to_string(&value.route_towards)?,
            route_back: serde_json::to_string(&value.route_back)?,
            state: value.state.clone() as u32,
        })
    }
}

impl TryFrom<&Traceroute> for crate::types::Traceroute {
    type Error = anyhow::Error;

    fn try_from(value: &Traceroute) -> Result<Self, Self::Error> {
        Ok(Self {
            message_id: value.message_id,
            node_key: value.node_key,
            datetime: value.datetime,
            routing_error: value
                .routing_error
                .map(meshtastic::protobufs::routing::Error::try_from)
                .transpose()?,
            duration: TimeDelta::milliseconds(value.duration_ms),
            route_towards: serde_json::from_str(&value.route_towards)?,
            route_back: serde_json::from_str(&value.route_back)?,
            state: TracerouteState::from_repr(value.state).ok_or(anyhow::anyhow!("invalid TracerouteState value"))?,
        })
    }
}
