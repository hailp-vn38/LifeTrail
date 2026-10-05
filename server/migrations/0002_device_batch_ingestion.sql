CREATE TABLE ingest_batches (
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE RESTRICT,
    batch_id UUID NOT NULL,
    schema_name TEXT NOT NULL CHECK (schema_name = 'gps/1'),
    content_sha256 BYTEA NOT NULL CHECK (octet_length(content_sha256) = 32),
    byte_length BIGINT NOT NULL CHECK (byte_length > 0),
    record_count BIGINT NOT NULL CHECK (record_count > 0),
    first_ts_ms BIGINT NOT NULL CHECK (first_ts_ms > 0),
    last_ts_ms BIGINT NOT NULL CHECK (last_ts_ms >= first_ts_ms),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (device_id, batch_id)
);

CREATE TABLE gps_points (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    device_id UUID NOT NULL,
    batch_id UUID NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL,
    lat DOUBLE PRECISION NOT NULL CHECK (lat >= -90 AND lat <= 90),
    lon DOUBLE PRECISION NOT NULL CHECK (lon >= -180 AND lon <= 180),
    alt_m DOUBLE PRECISION,
    speed_mps DOUBLE PRECISION CHECK (speed_mps >= 0),
    course_deg DOUBLE PRECISION CHECK (course_deg >= 0 AND course_deg < 360),
    fix_quality BIGINT NOT NULL CHECK (fix_quality >= 0),
    satellites BIGINT NOT NULL CHECK (satellites >= 0),
    hdop DOUBLE PRECISION CHECK (hdop >= 0),
    geometry geometry(Point, 4326) NOT NULL,
    FOREIGN KEY (device_id, batch_id)
        REFERENCES ingest_batches(device_id, batch_id)
        ON DELETE RESTRICT
);

CREATE INDEX gps_points_device_id_recorded_at_idx ON gps_points (device_id, recorded_at);
