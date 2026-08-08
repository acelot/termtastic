use super::Node;
use crate::repository::{Repository, RepositoryError};
use rusqlite::params;
use serde_rusqlite::{from_row, from_rows, to_params_named};

pub const TABLE_NAME: &str = "nodes";

impl Repository {
    pub async fn node_find_all(&self) -> Result<Vec<Node>, RepositoryError> {
        let result: Vec<Node> = self
            .conn
            .call(|conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!("SELECT * FROM {}", TABLE_NAME))?;
                let result: Result<_, _> = from_rows::<Node>(statement.query([])?).collect();

                result.map_err(Into::into)
            })
            .await?;

        Ok(result)
    }

    pub async fn node_get_by_key(&self, key: u32) -> Result<Option<Node>, RepositoryError> {
        let result: Option<Node> = self
            .conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!("SELECT * FROM {} WHERE key = :key", TABLE_NAME))?;

                match statement.query_one(params![key], |row| Ok(from_row::<Node>(row))) {
                    Ok(Ok(node)) => Ok(Some(node)),
                    Ok(Err(e)) => Err(e.into()),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e.into()),
                }
            })
            .await?;

        Ok(result)
    }

    pub async fn node_upsert(&self, node: Node) -> Result<(), RepositoryError> {
        self.conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!(
                    r#"
                    INSERT INTO {} (
                        key,
                        hops,
                        last_heard,
                        snr,
                        rssi,
                        is_favorite,
                        is_ignored,
                        is_muted,
                        user_id,
                        user_short_name,
                        user_long_name,
                        user_role,
                        user_hw_model,
                        user_public_key,
                        user_is_licensed,
                        user_is_unmessagable
                    )
                    VALUES (
                        :key,
                        :hops,
                        :last_heard,
                        :snr,
                        :rssi,
                        :is_favorite,
                        :is_ignored,
                        :is_muted,
                        :user_id,
                        :user_short_name,
                        :user_long_name,
                        :user_role,
                        :user_hw_model,
                        :user_public_key,
                        :user_is_licensed,
                        :user_is_unmessagable
                    )
                    ON CONFLICT(key) DO UPDATE SET
                        hops = excluded.hops,
                        last_heard = excluded.last_heard,
                        snr = excluded.snr,
                        rssi = excluded.rssi,
                        is_favorite = excluded.is_favorite,
                        is_ignored = excluded.is_ignored,
                        is_muted = excluded.is_muted,
                        user_id = excluded.user_id,
                        user_short_name = excluded.user_short_name,
                        user_long_name = excluded.user_long_name,
                        user_role = excluded.user_role,
                        user_hw_model = excluded.user_hw_model,
                        user_public_key = excluded.user_public_key,
                        user_is_licensed = excluded.user_is_licensed,
                        user_is_unmessagable = excluded.user_is_unmessagable
                    WHERE excluded.last_heard >= last_heard
                    "#,
                    TABLE_NAME
                ))?;

                statement
                    .execute(to_params_named(&node)?.to_slice().as_slice())
                    .map_err(|e| e.into())
            })
            .await?;

        Ok(())
    }

    pub async fn node_remove(&self, key: u32) -> Result<(), RepositoryError> {
        self.conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!("DELETE FROM {} WHERE key = :key", TABLE_NAME))?;
                statement.execute(params![key]).map_err(|e| e.into())
            })
            .await?;

        Ok(())
    }
}
