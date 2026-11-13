use crate::repository::domain::message::Message;
use crate::repository::{Repository, RepositoryError};
use crate::types::Chat;
use rusqlite::named_params;
use serde_rusqlite::{from_rows, to_params_named};
use uuid::Uuid;

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
                                ORDER BY id DESC
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
                    JOIN messages AS e ON e.reply_id = r.id AND e.is_emoji = 1

                    ORDER BY chat_id, is_emoji, id
                    "#,
                )?;

                let result: Result<_, _> =
                    from_rows::<Message>(statement.query(named_params! {":limit_per_chat": limit_per_chat})?).collect();

                result.map_err(Into::into)
            })
            .await?;

        Ok(result)
    }

    pub async fn message_find_before_datetime_for_chat(
        &self,
        chat: &Chat,
        before_id: Uuid,
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
                        WHERE chat_id = :chat_id AND id < :before_id AND is_emoji = 0
                        ORDER BY id DESC
                        LIMIT :limit
                    )
                    SELECT m.*
                    FROM chunk
                    JOIN messages AS m ON m.id = chunk.id

                    UNION ALL

                    SELECT e.*
                    FROM chunk
                    JOIN messages AS e ON e.reply_id = chunk.id AND e.is_emoji = 1

                    ORDER BY is_emoji, id
                    "#,
                )?;

                let result: Result<_, _> = from_rows::<Message>(statement.query(named_params! {
                    ":chat_id": chat_id,
                    ":before_id": before_id,
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
                        reply_id,
                        packet_id,
                        reply_packet_id,
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
                        :reply_id,
                        :packet_id,
                        :reply_packet_id,
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
                        reply_id = excluded.reply_id,
                        packet_id = excluded.packet_id,
                        reply_packet_id = excluded.reply_packet_id,
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
        id: Uuid,
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

    pub async fn message_remove_by_chat(&self, chat: Chat) -> Result<(), RepositoryError> {
        self.conn
            .call(move |conn| -> Result<_, RepositoryError> {
                let chat_id = match chat {
                    Chat::Channel(channel_key) => channel_key,
                    Chat::Direct(node_key) => node_key,
                };

                let mut statement = conn.prepare(
                    r#"
                    DELETE FROM messages WHERE chat_id = :chat_id
                    "#,
                )?;

                statement
                    .execute(named_params! { ":chat_id": chat_id })
                    .map_err(Into::into)
            })
            .await?;

        Ok(())
    }
}
