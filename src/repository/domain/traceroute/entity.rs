use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Traceroute {
    pub message_id: u32,
    pub node_key: u32,
    pub datetime: DateTime<Utc>,
    pub routing_error: Option<i32>,
    pub duration_ms: i64,
    pub route_towards: String,
    pub route_back: String,
    pub state: u32,
}
