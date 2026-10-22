use crate::repository::domain::position::Position;
use crate::repository::{Repository, RepositoryError};
use rusqlite::params;
use serde_rusqlite::{from_rows, to_params_named};
use std::collections::HashMap;

pub const TABLE_NAME: &str = "positions";

impl Repository {
    #[allow(dead_code)]
    pub async fn position_find_last_for_each_node(&self) -> Result<HashMap<u32, Position>, RepositoryError> {
        let result: HashMap<u32, Position> = self
            .conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!(
                    r#"
                    SELECT *
                    FROM (
                        SELECT p.*, ROW_NUMBER() OVER (
                            PARTITION BY node_key
                            ORDER BY datetime DESC
                        ) AS rn
                        FROM {} p
                    )
                    WHERE rn = 1;
                    "#,
                    TABLE_NAME
                ))?;

                from_rows::<Position>(statement.query([])?)
                    .map(|item| item.map(|p| (p.node_key, p)))
                    .collect::<Result<HashMap<u32, Position>, _>>()
                    .map_err(Into::into)
            })
            .await?;

        Ok(result)
    }

    pub async fn position_insert(&self, position: Position) -> Result<i64, RepositoryError> {
        let result: i64 = self
            .conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!(
                    r#"
                    INSERT INTO {} (
                        node_key,
                        datetime,
                        latitude_i,
                        longitude_i,
                        altitude,
                        time,
                        location_source,
                        altitude_source,
                        timestamp,
                        timestamp_millis_adjust,
                        altitude_hae,
                        altitude_geoidal_separation,
                        pdop,
                        hdop,
                        vdop,
                        gps_accuracy,
                        ground_speed,
                        ground_track,
                        fix_quality,
                        fix_type,
                        sats_in_view,
                        sensor_id,
                        next_update,
                        seq_number,
                        precision_bits
                    )
                    VALUES (
                        :node_key,
                        :datetime,
                        :latitude_i,
                        :longitude_i,
                        :altitude,
                        :time,
                        :location_source,
                        :altitude_source,
                        :timestamp,
                        :timestamp_millis_adjust,
                        :altitude_hae,
                        :altitude_geoidal_separation,
                        :pdop,
                        :hdop,
                        :vdop,
                        :gps_accuracy,
                        :ground_speed,
                        :ground_track,
                        :fix_quality,
                        :fix_type,
                        :sats_in_view,
                        :sensor_id,
                        :next_update,
                        :seq_number,
                        :precision_bits
                    )
                    "#,
                    TABLE_NAME
                ))?;

                statement
                    .insert(to_params_named(&position)?.to_slice().as_slice())
                    .map_err(Into::into)
            })
            .await?;

        Ok(result)
    }

    #[allow(dead_code)]
    pub async fn position_remove_by_node_key(&self, node_key: u32) -> Result<(), RepositoryError> {
        self.conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!("DELETE FROM {} WHERE node_key = :node_key", TABLE_NAME))?;
                statement.execute(params![node_key]).map_err(Into::into)
            })
            .await?;

        Ok(())
    }
}
