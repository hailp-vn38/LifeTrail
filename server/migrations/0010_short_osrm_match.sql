-- Matcher policy is a processing target input.  A new immutable revision is
-- required whenever its decision or identity changes.
ALTER TABLE device_processing_control
    ALTER COLUMN target_id SET DEFAULT 'osrm-match-v1',
    ADD COLUMN match_min_confidence DOUBLE PRECISION NOT NULL DEFAULT 0.70 CHECK (match_min_confidence >= 0 AND match_min_confidence <= 1),
    ADD COLUMN match_max_attempts BIGINT NOT NULL DEFAULT 2 CHECK (match_max_attempts > 0),
    ADD COLUMN match_retry_delay_ms BIGINT NOT NULL DEFAULT 50 CHECK (match_retry_delay_ms >= 0),
    ADD COLUMN match_total_budget_ms BIGINT NOT NULL DEFAULT 1000 CHECK (match_total_budget_ms > 0),
    ADD COLUMN matcher_engine_id TEXT NOT NULL DEFAULT 'osrm-v6',
    ADD COLUMN matcher_dataset_id TEXT NOT NULL DEFAULT 'configured-dataset';
UPDATE device_processing_control SET target_id='osrm-match-v1', target_generation=target_generation+1;
CREATE FUNCTION advance_osrm_match_target() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF (NEW.match_min_confidence, NEW.match_max_attempts, NEW.match_retry_delay_ms,
        NEW.match_total_budget_ms, NEW.matcher_engine_id, NEW.matcher_dataset_id)
        IS DISTINCT FROM
       (OLD.match_min_confidence, OLD.match_max_attempts, OLD.match_retry_delay_ms,
        OLD.match_total_budget_ms, OLD.matcher_engine_id, OLD.matcher_dataset_id) THEN
        NEW.target_generation := OLD.target_generation + 1;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER osrm_match_config_target BEFORE UPDATE ON device_processing_control
FOR EACH ROW EXECUTE FUNCTION advance_osrm_match_target();
