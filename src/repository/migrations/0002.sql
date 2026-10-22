CREATE TABLE positions
(
    id                          INTEGER PRIMARY KEY ASC,
    node_key                    INTEGER NOT NULL,
    datetime                    INTEGER NOT NULL,
    latitude_i                  INTEGER,
    longitude_i                 INTEGER,
    altitude                    INTEGER,
    time                        INTEGER NOT NULL,
    location_source             INTEGER NOT NULL,
    altitude_source             INTEGER NOT NULL,
    timestamp                   INTEGER NOT NULL,
    timestamp_millis_adjust     INTEGER NOT NULL,
    altitude_hae                INTEGER,
    altitude_geoidal_separation INTEGER,
    pdop                        INTEGER NOT NULL,
    hdop                        INTEGER NOT NULL,
    vdop                        INTEGER NOT NULL,
    gps_accuracy                INTEGER NOT NULL,
    ground_speed                INTEGER,
    ground_track                INTEGER,
    fix_quality                 INTEGER NOT NULL,
    fix_type                    INTEGER NOT NULL,
    sats_in_view                INTEGER NOT NULL,
    sensor_id                   INTEGER NOT NULL,
    next_update                 INTEGER NOT NULL,
    seq_number                  INTEGER NOT NULL,
    precision_bits              INTEGER NOT NULL
);

CREATE INDEX idx_positions_node_key_id ON positions (node_key, datetime DESC);
