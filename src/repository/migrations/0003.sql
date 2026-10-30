ALTER TABLE messages ADD COLUMN chat_id INTEGER
GENERATED ALWAYS AS (
    CASE
        WHEN "to" IN (0, 4294967295) THEN channel
        WHEN "to" = :node_key THEN "from"
        ELSE "to"
    END
) VIRTUAL;

CREATE INDEX idx_messages_chat_id_datetime_id ON messages(chat_id, datetime DESC, id DESC) WHERE is_emoji = 0;
CREATE INDEX idx_messages_reply_message_id_is_emoji ON messages(reply_message_id) WHERE is_emoji = 1;

DROP INDEX idx_messages_datetime;
DROP INDEX idx_messages_channel_to;
DROP INDEX idx_messages_from_to;