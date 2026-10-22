use crate::repository::domain::position::Position;
use crate::types::NodePosition;

impl From<NodePosition> for Position {
    fn from(value: NodePosition) -> Self {
        let p = value.position;

        Self {
            id: None,
            node_key: value.node_key,
            datetime: value.datetime,
            latitude_i: p.latitude_i,
            longitude_i: p.longitude_i,
            altitude: p.altitude,
            time: p.time,
            location_source: p.location_source,
            altitude_source: p.altitude_source,
            timestamp: p.timestamp,
            timestamp_millis_adjust: p.timestamp_millis_adjust,
            altitude_hae: p.altitude_hae,
            altitude_geoidal_separation: p.altitude_geoidal_separation,
            pdop: p.pdop,
            hdop: p.hdop,
            vdop: p.vdop,
            gps_accuracy: p.gps_accuracy,
            ground_speed: p.ground_speed,
            ground_track: p.ground_track,
            fix_quality: p.fix_quality,
            fix_type: p.fix_type,
            sats_in_view: p.sats_in_view,
            sensor_id: p.sensor_id,
            next_update: p.next_update,
            seq_number: p.seq_number,
            precision_bits: p.precision_bits,
        }
    }
}

impl From<&Position> for NodePosition {
    fn from(value: &Position) -> Self {
        Self {
            node_key: value.node_key,
            datetime: value.datetime,
            position: meshtastic::protobufs::Position {
                latitude_i: value.latitude_i,
                longitude_i: value.longitude_i,
                altitude: value.altitude,
                time: value.time,
                location_source: value.location_source,
                altitude_source: value.altitude_source,
                timestamp: value.timestamp,
                timestamp_millis_adjust: value.timestamp_millis_adjust,
                altitude_hae: value.altitude_hae,
                altitude_geoidal_separation: value.altitude_geoidal_separation,
                pdop: value.pdop,
                hdop: value.hdop,
                vdop: value.vdop,
                gps_accuracy: value.gps_accuracy,
                ground_speed: value.ground_speed,
                ground_track: value.ground_track,
                fix_quality: value.fix_quality,
                fix_type: value.fix_type,
                sats_in_view: value.sats_in_view,
                sensor_id: value.sensor_id,
                next_update: value.next_update,
                seq_number: value.seq_number,
                precision_bits: value.precision_bits,
            },
        }
    }
}
