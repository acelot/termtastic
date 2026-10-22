use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Position {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub node_key: u32,
    pub datetime: DateTime<Utc>,
    pub latitude_i: Option<i32>,
    pub longitude_i: Option<i32>,
    pub altitude: Option<i32>,
    pub time: u32,
    pub location_source: i32,
    pub altitude_source: i32,
    pub timestamp: u32,
    pub timestamp_millis_adjust: i32,
    pub altitude_hae: Option<i32>,
    pub altitude_geoidal_separation: Option<i32>,
    pub pdop: u32,
    pub hdop: u32,
    pub vdop: u32,
    pub gps_accuracy: u32,
    pub ground_speed: Option<u32>,
    pub ground_track: Option<u32>,
    pub fix_quality: u32,
    pub fix_type: u32,
    pub sats_in_view: u32,
    pub sensor_id: u32,
    pub next_update: u32,
    pub seq_number: u32,
    pub precision_bits: u32,
}
