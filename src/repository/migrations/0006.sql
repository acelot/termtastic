CREATE TABLE chat_position
(
    chat_id     INTEGER PRIMARY KEY NOT NULL,
    selected_id BLOB NOT NULL,
    max_id      BLOB NOT NULL
) WITHOUT ROWID;