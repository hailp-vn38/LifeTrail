-- Bound quality failures independently of the threshold for true Raw GPS gaps.
ALTER TABLE device_processing_control
    ALTER COLUMN target_id SET DEFAULT 'short-evidence-continuity-v1',
    ADD COLUMN short_failure_max_s BIGINT NOT NULL DEFAULT 10
        CHECK (short_failure_max_s >= 0);
UPDATE device_processing_control
SET target_id='short-evidence-continuity-v1', target_generation=target_generation+1;
CREATE FUNCTION advance_continuity_target() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.short_failure_max_s IS DISTINCT FROM OLD.short_failure_max_s THEN
        NEW.target_generation := OLD.target_generation + 1;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER continuity_config_target BEFORE UPDATE ON device_processing_control
FOR EACH ROW EXECUTE FUNCTION advance_continuity_target();
