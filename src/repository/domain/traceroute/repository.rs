use crate::repository::domain::traceroute::Traceroute;
use crate::repository::{Repository, RepositoryError};
use serde_rusqlite::{from_rows, to_params_named};

pub const TABLE_NAME: &str = "traceroutes";

impl Repository {
    pub async fn traceroute_find_all(&self) -> Result<Vec<Traceroute>, RepositoryError> {
        let result: Vec<Traceroute> = self
            .conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!(
                    "SELECT * FROM {} ORDER BY node_key, datetime DESC",
                    TABLE_NAME
                ))?;

                let result: Result<_, _> = from_rows::<Traceroute>(statement.query([])?).collect();

                result.map_err(Into::into)
            })
            .await?;

        Ok(result)
    }

    pub async fn traceroute_upsert(&self, traceroute: Traceroute) -> Result<i64, RepositoryError> {
        let result: i64 = self
            .conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!(
                    r#"
                    INSERT INTO {} (
                        message_id,
                        node_key,
                        datetime,
                        routing_error,
                        duration_ms,
                        route_towards,
                        route_back,
                        state
                    )
                    VALUES (
                        :message_id,
                        :node_key,
                        :datetime,
                        :routing_error,
                        :duration_ms,
                        :route_towards,
                        :route_back,
                        :state
                    )
                    ON CONFLICT(message_id) DO UPDATE SET 
                        node_key = excluded.node_key,
                        datetime = excluded.datetime,
                        routing_error = excluded.routing_error,
                        duration_ms = excluded.duration_ms,
                        route_towards = excluded.route_towards,
                        route_back = excluded.route_back,
                        state = excluded.state
                    "#,
                    TABLE_NAME
                ))?;

                statement
                    .insert(to_params_named(&traceroute)?.to_slice().as_slice())
                    .map_err(Into::into)
            })
            .await?;

        Ok(result)
    }
}
