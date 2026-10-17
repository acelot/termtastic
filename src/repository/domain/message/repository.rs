use crate::repository::domain::message::Message;
use crate::repository::{Repository, RepositoryError};
use crate::types::Chat;
use rusqlite::{named_params, params};
use serde_rusqlite::{from_rows, to_params_named};

pub const TABLE_NAME: &str = "messages";

impl Repository {
    #[allow(dead_code)]
    pub async fn message_find_all(&self) -> Result<Vec<Message>, RepositoryError> {
        let result: Vec<Message> = self
            .conn
            .call(|conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!("SELECT * FROM {} ORDER BY datetime ASC", TABLE_NAME))?;
                let result: Result<_, _> = from_rows::<Message>(statement.query([])?).collect();

                result.map_err(Into::into)
            })
            .await?;

        Ok(result)
    }

    #[allow(dead_code)]
    pub async fn message_upsert(&self, message: Message) -> Result<(), RepositoryError> {
        self.conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!(
                    r#"
                    INSERT INTO {} (
                        id,
                        reply_message_id,
                        "from",
                        "to",
                        channel,
                        datetime,
                        text,
                        is_emoji,
                        hops,
                        snr,
                        rssi,
                        routing_error
                    )
                    VALUES (
                        :id,
                        :reply_message_id,
                        :from,
                        :to,
                        :channel,
                        :datetime,
                        :text,
                        :is_emoji,
                        :hops,
                        :snr,
                        :rssi,
                        :routing_error
                    )
                    ON CONFLICT(id) DO UPDATE SET
                        reply_message_id = excluded.reply_message_id,
                        "from" = excluded."from",
                        "to" = excluded."to",
                        channel = excluded.channel,
                        datetime = excluded.datetime,
                        text = excluded.text,
                        is_emoji = excluded.is_emoji,
                        hops = excluded.hops,
                        snr = excluded.snr,
                        rssi = excluded.rssi,
                        routing_error = excluded.routing_error
                    "#,
                    TABLE_NAME
                ))?;

                statement
                    .execute(to_params_named(&message)?.to_slice().as_slice())
                    .map_err(Into::into)
            })
            .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn message_update_routing_error(
        &self,
        id: u32,
        routing_error: Option<i32>,
    ) -> Result<(), RepositoryError> {
        self.conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!(
                    "UPDATE {} SET routing_error = :routing_error WHERE id = :id",
                    TABLE_NAME
                ))?;

                statement
                    .execute(named_params! { ":id": id, ":routing_error": routing_error })
                    .map_err(Into::into)
            })
            .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn message_remove(&self, id: u32) -> Result<(), RepositoryError> {
        self.conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(&format!("DELETE FROM {} WHERE id = :id", TABLE_NAME))?;
                statement.execute(params![id]).map_err(Into::into)
            })
            .await?;

        Ok(())
    }

    /// Removes all messages of the chat. Follows the same logic as `Chat::from((&Message, my_node_key))`.
    #[allow(dead_code)]
    pub async fn message_remove_by_chat(&self, chat: Chat, my_node_key: u32) -> Result<(), RepositoryError> {
        self.conn
            .call(move |conn| -> Result<_, RepositoryError> {
                match chat {
                    Chat::Channel(channel_key) => {
                        let mut statement = conn.prepare(&format!(
                            r#"DELETE FROM {} WHERE channel = :channel AND "to" IN (0, :broadcast)"#,
                            TABLE_NAME
                        ))?;

                        statement.execute(named_params! { ":channel": channel_key, ":broadcast": u32::MAX })
                    }
                    Chat::Direct(node_key) => {
                        let mut statement = conn.prepare(&format!(
                            r#"
                            DELETE FROM {}
                            WHERE ("from" = :me AND "to" = :node) OR ("from" = :node AND "to" = :me)
                            "#,
                            TABLE_NAME
                        ))?;

                        statement.execute(named_params! { ":me": my_node_key, ":node": node_key })
                    }
                }
                .map_err(Into::into)
            })
            .await?;

        Ok(())
    }
}
