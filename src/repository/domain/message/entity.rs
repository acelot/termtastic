use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Message {
    pub id: u32,
    pub reply_message_id: u32,
    pub from: u32,
    pub to: u32,
    pub channel: u32,
    pub datetime: DateTime<Utc>,
    pub text: String,
    pub is_emoji: bool,
    pub hops: u32,
    pub snr: f32,
    pub rssi: i32,
    pub routing_error: Option<i32>,
}
