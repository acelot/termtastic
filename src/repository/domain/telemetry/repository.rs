use crate::repository::domain::telemetry::Telemetry;
use crate::repository::{Repository, RepositoryError};
use rusqlite::params;
use serde_rusqlite::{from_rows, to_params_named};
use std::collections::HashMap;

pub const TABLE_NAME: &str = "telemetry";

impl Repository {
    #[allow(dead_code)]
    pub async fn telemetry_find_all(&self) -> Result<Vec<Telemetry>, RepositoryError> {
        let result: Vec<Telemetry> = self
            .conn
            .call(|conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!("SELECT * FROM {}", TABLE_NAME))?;
                let result: Result<_, _> = from_rows::<Telemetry>(statement.query([])?).collect();

                result.map_err(Into::into)
            })
            .await?;

        Ok(result)
    }

    #[allow(dead_code)]
    pub async fn telemetry_find_last_by_node_key(&self, node_key: u32) -> Result<Vec<Telemetry>, RepositoryError> {
        let result: Vec<Telemetry> = self
            .conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!(
                    r#"
                    SELECT *
                    FROM (
                        SELECT t.*, ROW_NUMBER() OVER (
                            PARTITION BY node_key, kind
                            ORDER BY datetime DESC
                        ) AS rn
                        FROM {} t WHERE t.node_key = :node_key
                    )
                    WHERE rn = 1;
                    "#,
                    TABLE_NAME
                ))?;

                let result: Result<_, _> = from_rows::<Telemetry>(statement.query(params![node_key])?)
                    .collect();

                result.map_err(Into::into)
            })
            .await?;

        Ok(result)
    }

    pub async fn telemetry_find_last_for_each_node(&self) -> Result<HashMap<u32, Vec<Telemetry>>, RepositoryError> {
        let result: HashMap<u32, Vec<Telemetry>> = self
            .conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!(
                    r#"
                    SELECT *
                    FROM (
                        SELECT t.*, ROW_NUMBER() OVER (
                            PARTITION BY node_key, kind
                            ORDER BY datetime DESC
                        ) AS rn
                        FROM {} t
                    )
                    WHERE rn = 1;
                    "#,
                    TABLE_NAME
                ))?;

                from_rows::<Telemetry>(statement.query([])?)
                    .collect::<Result<Vec<Telemetry>, _>>()
                    .map(|valid_items| {
                        valid_items
                            .into_iter()
                            .fold(HashMap::new(), |mut map: HashMap<u32, Vec<Telemetry>>, item| {
                                map.entry(item.node_key).or_default().push(item);
                                map
                            })
                    })
                    .map_err(Into::into)
            })
            .await?;

        Ok(result)
    }

    pub async fn telemetry_insert(&self, telemetry: Telemetry) -> Result<i64, RepositoryError> {
        let result: i64 = self
            .conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!(
                    r#"
                    INSERT INTO {} (
                        node_key,
                        datetime,
                        kind,
                        data
                    )
                    VALUES (
                        :node_key,
                        :datetime,
                        :kind,
                        :data
                    )
                    "#,
                    TABLE_NAME
                ))?;

                statement
                    .insert(to_params_named(&telemetry)?.to_slice().as_slice())
                    .map_err(Into::into)
            })
            .await?;

        Ok(result)
    }

    #[allow(dead_code)]
    pub async fn telemetry_remove_by_node_key(&self, node_key: u32) -> Result<(), RepositoryError> {
        self.conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!("DELETE FROM {} WHERE node_key = :node_key", TABLE_NAME))?;
                statement.execute(params![node_key]).map_err(Into::into)
            })
            .await?;

        Ok(())
    }
}
