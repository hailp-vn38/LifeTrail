-- Quality classification, GPS Gaps and Evidence Holes.
--
-- The policy that rejects unreliable or impossible observations becomes
-- centrally configurable. Changing it advances the processing target
-- generation, so publication built by the earlier reducer reports as stale
-- until it is rebuilt. Raw GPS is untouched by this migration.
ALTER TABLE device_processing_control
    ALTER COLUMN target_id SET DEFAULT 'quality-gaps-v1',
    ADD COLUMN max_hdop DOUBLE PRECISION NOT NULL DEFAULT 5 CHECK (max_hdop > 0),
    ADD COLUMN max_implied_speed_mps DOUBLE PRECISION NOT NULL DEFAULT 70 CHECK (max_implied_speed_mps > 0),
    ADD COLUMN jump_distance_floor_m DOUBLE PRECISION NOT NULL DEFAULT 100 CHECK (jump_distance_floor_m > 0);
UPDATE device_processing_control SET target_id='quality-gaps-v1', target_generation=target_generation+1;
CREATE FUNCTION advance_quality_target() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF (NEW.max_hdop, NEW.max_implied_speed_mps, NEW.jump_distance_floor_m)
        IS DISTINCT FROM (OLD.max_hdop, OLD.max_implied_speed_mps, OLD.jump_distance_floor_m) THEN
        NEW.target_generation := OLD.target_generation + 1;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER quality_config_target BEFORE UPDATE ON device_processing_control
FOR EACH ROW EXECUTE FUNCTION advance_quality_target();