use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::types::TelemetryKind;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Telemetry {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    pub id: Option<i64>,
    pub node_key: u32,
    pub datetime: Timestamp,
    pub kind: TelemetryKind,
    pub data: Vec<u8>,
}
