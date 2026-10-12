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