ALTER TABLE users ADD COLUMN timezone_generation BIGINT NOT NULL DEFAULT 0;
CREATE FUNCTION advance_timezone_generation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.timezone IS DISTINCT FROM OLD.timezone THEN
        NEW.timezone_generation := OLD.timezone_generation + 1;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER owner_timezone_generation BEFORE UPDATE OF timezone ON users
FOR EACH ROW EXECUTE FUNCTION advance_timezone_generation();

CREATE TABLE device_processing_control (
    device_id UUID PRIMARY KEY REFERENCES devices(id),
    input_generation BIGINT NOT NULL DEFAULT 0 CHECK (input_generation >= 0),
    target_generation BIGINT NOT NULL DEFAULT 1,
    target_id TEXT NOT NULL DEFAULT 'sparse-v1',
    fencing_token BIGINT NOT NULL DEFAULT 0
);
INSERT INTO device_processing_control (device_id, input_generation)
SELECT devices.id, count(ingest_batches.batch_id) FROM devices
LEFT JOIN ingest_batches ON ingest_batches.device_id = devices.id GROUP BY devices.id;

CREATE TABLE processing_jobs (
    device_id UUID PRIMARY KEY REFERENCES device_processing_control(device_id),
    state TEXT NOT NULL CHECK (state IN ('queued','running','idle','failed')),
    fencing_token BIGINT NOT NULL DEFAULT 0,
    lease_until TIMESTAMPTZ,
    failure_message TEXT
);
CREATE TABLE processing_days (
    device_id UUID NOT NULL REFERENCES device_processing_control(device_id),
    local_date DATE NOT NULL,
    timezone TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('queued','done','deferred')),
    PRIMARY KEY (device_id, local_date, timezone)
);
CREATE TABLE activity_manifests (
    id UUID PRIMARY KEY,
    device_id UUID NOT NULL REFERENCES devices(id),
    input_generation BIGINT NOT NULL,
    target_id TEXT NOT NULL,
    entries JSONB NOT NULL CHECK (jsonb_typeof(entries) = 'array'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(device_id, id)
);
CREATE TABLE daily_snapshots (
    id UUID PRIMARY KEY,
    device_id UUID NOT NULL,
    manifest_id UUID NOT NULL,
    local_date DATE NOT NULL,
    timezone TEXT NOT NULL,
    timezone_generation BIGINT NOT NULL,
    source_generation BIGINT NOT NULL,
    target_generation BIGINT NOT NULL,
    body JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (device_id, manifest_id) REFERENCES activity_manifests(device_id, id),
    UNIQUE(device_id, local_date, timezone, id)
);
CREATE TABLE daily_publications (
    device_id UUID NOT NULL,
    local_date DATE NOT NULL,
    timezone TEXT NOT NULL,
    snapshot_id UUID NOT NULL,
    PRIMARY KEY(device_id, local_date, timezone),
    FOREIGN KEY(device_id, local_date, timezone, snapshot_id)
        REFERENCES daily_snapshots(device_id, local_date, timezone, id)
);
CREATE FUNCTION protect_processing_history() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'processing history is immutable; create a new version';
END $$;
CREATE TRIGGER immutable_activity_manifest BEFORE UPDATE OR DELETE ON activity_manifests
FOR EACH ROW EXECUTE FUNCTION protect_processing_history();
CREATE TRIGGER immutable_daily_snapshot BEFORE UPDATE OR DELETE ON daily_snapshots
FOR EACH ROW EXECUTE FUNCTION protect_processing_history();
