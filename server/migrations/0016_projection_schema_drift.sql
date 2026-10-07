-- Display-projection schema is a projection target, not Raw input.  Reprojection
-- reuses the active immutable manifest and never re-runs the matcher, so it
-- leaves input_generation, timezone_generation and Activity Revisions untouched.
--
-- One row per accepted (device, day, target schema) request is the durable proof
-- the request was enqueued.  The primary key makes enqueuing the same
-- reprojection again a no-op, so a repeated request never produces a second
-- snapshot.
CREATE TABLE projection_requeues (
    device_id UUID NOT NULL REFERENCES device_processing_control(device_id),
    local_date DATE NOT NULL,
    target_schema_version INTEGER NOT NULL CHECK (target_schema_version >= 1),
    requested_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (device_id, local_date, target_schema_version)
);
