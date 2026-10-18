CREATE TABLE nodes
(
    key                  INTEGER PRIMARY KEY NOT NULL,
    hops                 INTEGER,
    last_heard           INTEGER,
    snr                  REAL                NOT NULL,
    rssi                 INTEGER,
    is_favorite          INTEGER             NOT NULL,
    is_ignored           INTEGER             NOT NULL,
    is_muted             INTEGER             NOT NULL,
    has_user             INTEGER             NOT NULL,
    user_id              TEXT                NOT NULL,
    user_short_name      TEXT                NOT NULL,
    user_long_name       TEXT                NOT NULL,
    user_role            INTEGER             NOT NULL,
    user_hw_model        INTEGER             NOT NULL,
    user_public_key      BLOB                NOT NULL,
    user_is_licensed     INTEGER             NOT NULL,
    user_is_unmessagable INTEGER
);

--

CREATE TABLE telemetry
(
    id       INTEGER PRIMARY KEY ASC,
    node_key INTEGER NOT NULL,
    datetime INTEGER NOT NULL,
    kind     TEXT    NOT NULL,
    data     BLOB
);

CREATE INDEX idx_telemetry_node_key_datetime ON telemetry (node_key, datetime DESC);
CREATE INDEX idx_telemetry_node_key_kind_datetime ON telemetry (node_key, kind, datetime DESC);

--

CREATE TABLE traceroutes
(
    message_id    INTEGER PRIMARY KEY NOT NULL,
    node_key      INTEGER             NOT NULL,
    datetime      INTEGER             NOT NULL,
    routing_error INTEGER,
    duration_ms   INTEGER,
    route_towards TEXT                NOT NULL,
    route_back    TEXT                NOT NULL,
    state         INTEGER             NOT NULL
);

CREATE INDEX idx_traceroutes_node_key_datetime ON traceroutes (node_key, datetime DESC);

--

CREATE TABLE messages
(
    id               INTEGER PRIMARY KEY NOT NULL,
    reply_message_id INTEGER             NOT NULL,
    "from"           INTEGER             NOT NULL,
    "to"             INTEGER             NOT NULL,
    channel          INTEGER             NOT NULL,
    datetime         INTEGER             NOT NULL,
    text             TEXT                NOT NULL,
    is_emoji         INTEGER             NOT NULL,
    hops             INTEGER             NOT NULL,
    snr              REAL                NOT NULL,
    rssi             INTEGER             NOT NULL,
    routing_error    INTEGER
);

CREATE INDEX idx_messages_datetime ON messages (datetime);
CREATE INDEX idx_messages_channel_to ON messages (channel, "to");
CREATE INDEX idx_messages_from_to ON messages ("from", "to");