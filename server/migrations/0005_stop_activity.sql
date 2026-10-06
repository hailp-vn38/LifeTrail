ALTER TABLE device_processing_control
    ALTER COLUMN target_id SET DEFAULT 'stops-v1',
    ADD COLUMN stop_radius_m DOUBLE PRECISION NOT NULL DEFAULT 30 CHECK (stop_radius_m > 0 AND stop_radius_m < 10000),
    ADD COLUMN stop_min_duration_s BIGINT NOT NULL DEFAULT 180 CHECK (stop_min_duration_s > 0),
    ADD COLUMN observation_gap_s BIGINT NOT NULL DEFAULT 300 CHECK (observation_gap_s > 0);
UPDATE device_processing_control SET target_id='stops-v1', target_generation=target_generation+1;
CREATE FUNCTION advance_stop_target() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF (NEW.stop_radius_m, NEW.stop_min_duration_s, NEW.observation_gap_s)
        IS DISTINCT FROM (OLD.stop_radius_m, OLD.stop_min_duration_s, OLD.observation_gap_s) THEN
        NEW.target_generation := OLD.target_generation + 1;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER stop_config_target BEFORE UPDATE ON device_processing_control
FOR EACH ROW EXECUTE FUNCTION advance_stop_target();

CREATE TABLE activity_revisions (
    id UUID PRIMARY KEY,
    device_id UUID NOT NULL REFERENCES devices(id),
    observed_from_at TIMESTAMPTZ NOT NULL,
    observed_until_at TIMESTAMPTZ NOT NULL,
    input_generation BIGINT NOT NULL,
    target_generation BIGINT NOT NULL,
    config JSONB NOT NULL,
    body JSONB NOT NULL,
    CHECK (observed_until_at > observed_from_at),
    UNIQUE(device_id, id)
);
CREATE TRIGGER immutable_activity_revision BEFORE UPDATE OR DELETE ON activity_revisions
FOR EACH ROW EXECUTE FUNCTION protect_processing_history();
