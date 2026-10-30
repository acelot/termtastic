use crate::repository::domain::message::Message;
use crate::repository::{Repository, RepositoryError};
use crate::types::Chat;
use chrono::{DateTime, Utc};
use rusqlite::{named_params, params};
use serde_rusqlite::{from_rows, to_params_named};

impl Repository {
    pub async fn message_find_recent_for_each_chat(
        &self,
        limit_per_chat: u32,
    ) -> Result<Vec<Message>, RepositoryError> {
        let result: Vec<Message> = self
            .conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(
                    r#"
                    WITH r AS (
                        SELECT
                            id,
                            ROW_NUMBER() OVER (
                                PARTITION BY chat_id
                                ORDER BY datetime DESC, id DESC
                            ) AS rn
                        FROM messages
                        WHERE is_emoji = 0
                    )
                    SELECT m.*
                    FROM r
                    JOIN messages AS m ON r.id = m.id
                    WHERE r.rn <= :limit_per_chat

                    UNION ALL

                    SELECT e.*
                    FROM r
                    JOIN messages AS e ON e.reply_message_id = r.id AND e.is_emoji = 1
                    
                    ORDER BY datetime, id;
                    "#,
                )?;

                let result: Result<_, _> =
                    from_rows::<Message>(statement.query(named_params! {":limit_per_chat": limit_per_chat})?).collect();

                result.map_err(Into::into)
            })
            .await?;

        Ok(result)
    }

    #[allow(dead_code)]
    pub async fn message_find_before_datetime_for_chat(
        &self,
        chat: &Chat,
        before_datetime: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<Message>, RepositoryError> {
        let chat_id = match chat {
            Chat::Channel(ch) => *ch,
            Chat::Direct(node_key) => *node_key,
        };

        let result: Vec<Message> = self
            .conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(
                    r#"
                    WITH chunk AS (
                        SELECT id 
                        FROM messages
                        WHERE chat_id = :chat_id AND datetime < :before_datetime AND is_emoji = 0
                        ORDER BY datetime DESC, id DESC
                        LIMIT :limit
                    )
                    SELECT m.*
                    FROM chunk
                    JOIN messages AS m ON m.id = chunk.id

                    UNION ALL

                    SELECT e.*
                    FROM chunk
                    JOIN messages AS e ON e.reply_message_id = chunk.id AND e.is_emoji = 1
                    
                    ORDER BY datetime, id;
                    "#,
                )?;

                let result: Result<_, _> = from_rows::<Message>(statement.query(named_params! {
                    ":chat_id": chat_id,
                    ":before_datetime": before_datetime.timestamp(),
                    ":limit": limit
                })?)
                .collect();

                result.map_err(Into::into)
            })
            .await?;

        Ok(result)
    }

    pub async fn message_upsert(&self, message: Message) -> Result<(), RepositoryError> {
        self.conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement = conn.prepare(
                    r#"
                    INSERT INTO messages (
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
                )?;

                statement
                    .execute(to_params_named(&message)?.to_slice().as_slice())
                    .map_err(Into::into)
            })
            .await?;

        Ok(())
    }

    pub async fn message_update_routing_error(
        &self,
        id: u32,
        routing_error: Option<i32>,
    ) -> Result<(), RepositoryError> {
        self.conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let mut statement =
                    conn.prepare("UPDATE messages SET routing_error = :routing_error WHERE id = :id")?;

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
                let mut statement = conn.prepare("DELETE FROM messages WHERE id = :id")?;
                statement.execute(params![id]).map_err(Into::into)
            })
            .await?;

        Ok(())
    }

    pub async fn message_remove_by_chat(&self, chat: Chat, my_node_key: u32) -> Result<(), RepositoryError> {
        self.conn
            .call(move |conn| -> Result<_, RepositoryError> {
                match chat {
                    Chat::Channel(channel_key) => {
                        let mut statement = conn.prepare(
                            r#"
                            DELETE FROM messages 
                            WHERE channel = :channel AND "to" IN (0, :broadcast)
                            "#,
                        )?;

                        statement.execute(named_params! { ":channel": channel_key, ":broadcast": u32::MAX })
                    }
                    Chat::Direct(node_key) => {
                        let mut statement = conn.prepare(
                            r#"
                            DELETE FROM messages
                            WHERE ("from" = :me AND "to" = :node) OR ("from" = :node AND "to" = :me)
                            "#,
                        )?;

                        statement.execute(named_params! { ":me": my_node_key, ":node": node_key })
                    }
                }
                .map_err(Into::into)
            })
            .await?;

        Ok(())
    }
}
