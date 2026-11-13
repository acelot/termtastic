-- migrate messages
CREATE TABLE messages_new
(
    id               BLOB PRIMARY KEY NOT NULL,
    reply_id         BLOB NOT NULL DEFAULT (zeroblob(16)),
    packet_id        INTEGER NOT NULL,
    reply_packet_id  INTEGER NOT NULL,
    "from"           INTEGER NOT NULL,
    "to"             INTEGER NOT NULL,
    channel          INTEGER NOT NULL,
    datetime         INTEGER NOT NULL,
    text             TEXT    NOT NULL,
    is_emoji         INTEGER NOT NULL,
    hops             INTEGER NOT NULL,
    snr              REAL    NOT NULL,
    rssi             INTEGER NOT NULL,
    routing_error    INTEGER,
    chat_id          INTEGER AS (
        CASE
            WHEN "to" IN (0, 4294967295) THEN channel
            WHEN "to" = :node_key THEN "from"
            ELSE "to"
        END
    ) STORED
);

-- copy records from old table
INSERT INTO messages_new (
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
SELECT
    uuid_v7(unixepoch(datetime)) AS id,
    zeroblob(16) AS reply_id,
    id AS packet_id,
    reply_message_id AS reply_packet_id,
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
FROM messages ORDER BY datetime, id;

-- fill reply_id
UPDATE messages_new AS m
SET reply_id = (
    SELECT parent.id
    FROM messages_new AS parent
    WHERE parent.packet_id = m.reply_packet_id
)
WHERE m.reply_packet_id != 0 AND EXISTS (
    SELECT 1
    FROM messages_new AS parent
    WHERE parent.packet_id = m.reply_packet_id
);

-- swap tables
ALTER TABLE messages RENAME TO messages_old;
ALTER TABLE messages_new RENAME TO messages;

-- add indexes
CREATE INDEX idx_messages_chat_id_id ON messages(chat_id, id DESC) WHERE is_emoji = 0;
CREATE INDEX idx_messages_reply_id_is_emoji ON messages(reply_id) WHERE is_emoji = 1;