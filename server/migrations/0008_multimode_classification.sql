-- Windowed mode evidence is a captured processing input.  Any policy change
-- advances the target identity so immutable revisions remain reproducible.
ALTER TABLE device_processing_control
    ALTER COLUMN target_id SET DEFAULT 'multimode-v1',
    ADD COLUMN mode_window_s BIGINT NOT NULL DEFAULT 120 CHECK (mode_window_s > 0),
    ADD COLUMN mode_change_min_duration_s BIGINT NOT NULL DEFAULT 120 CHECK (mode_change_min_duration_s > 0),
    ADD COLUMN mode_enter_confidence DOUBLE PRECISION NOT NULL DEFAULT 0.70 CHECK (mode_enter_confidence > 0 AND mode_enter_confidence <= 1),
    ADD COLUMN mode_exit_confidence DOUBLE PRECISION NOT NULL DEFAULT 0.55 CHECK (mode_exit_confidence >= 0 AND mode_exit_confidence < mode_enter_confidence),
    ADD COLUMN mode_unknown_grace_s BIGINT NOT NULL DEFAULT 90 CHECK (mode_unknown_grace_s >= 0);
UPDATE device_processing_control SET target_id='multimode-v1', target_generation=target_generation+1;
CREATE FUNCTION advance_multimode_target() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF (NEW.mode_window_s, NEW.mode_change_min_duration_s, NEW.mode_enter_confidence, NEW.mode_exit_confidence, NEW.mode_unknown_grace_s)
        IS DISTINCT FROM (OLD.mode_window_s, OLD.mode_change_min_duration_s, OLD.mode_enter_confidence, OLD.mode_exit_confidence, OLD.mode_unknown_grace_s) THEN
        NEW.target_generation := OLD.target_generation + 1;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER multimode_config_target BEFORE UPDATE ON device_processing_control
FOR EACH ROW EXECUTE FUNCTION advance_multimode_target();
