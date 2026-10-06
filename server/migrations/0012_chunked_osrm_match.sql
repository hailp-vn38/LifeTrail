-- These values bound request size only.  They are deliberately retained in the
-- processing target because changing either can alter immutable Route Parts.
ALTER TABLE device_processing_control
    ADD COLUMN match_chunk_max_points BIGINT NOT NULL DEFAULT 80 CHECK (match_chunk_max_points >= 2),
    ADD COLUMN match_chunk_overlap_points BIGINT NOT NULL DEFAULT 5 CHECK (match_chunk_overlap_points >= 1 AND match_chunk_overlap_points < match_chunk_max_points);

ALTER TABLE device_processing_control
    ALTER COLUMN target_id SET DEFAULT 'chunked-osrm-match-v1';
UPDATE device_processing_control SET target_id='chunked-osrm-match-v1', target_generation=target_generation+1;

CREATE FUNCTION advance_chunked_osrm_match_target() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF (NEW.match_chunk_max_points, NEW.match_chunk_overlap_points)
        IS DISTINCT FROM (OLD.match_chunk_max_points, OLD.match_chunk_overlap_points) THEN
        NEW.target_generation := OLD.target_generation + 1;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER chunked_osrm_match_config_target BEFORE UPDATE ON device_processing_control
FOR EACH ROW EXECUTE FUNCTION advance_chunked_osrm_match_target();
