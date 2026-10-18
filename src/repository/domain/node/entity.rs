use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Node {
    pub key: u32,
    pub hops: Option<u32>,
    pub last_heard: Option<DateTime<Utc>>,
    pub snr: f32,
    pub rssi: Option<i32>,
    pub is_favorite: bool,
    pub is_ignored: bool,
    pub is_muted: bool,
    pub has_user: bool,
    pub user_id: String,
    pub user_short_name: String,
    pub user_long_name: String,
    pub user_role: i32,
    pub user_hw_model: i32,
    pub user_public_key: Vec<u8>,
    pub user_is_licensed: bool,
    pub user_is_unmessagable: Option<bool>,
}
