-- Supersede already committed matcher migrations without rewriting history.
-- Immutable existing snapshots/revisions remain available until reprocessing.
DROP TRIGGER osrm_match_config_target ON device_processing_control;
DROP TRIGGER chunked_osrm_match_config_target ON device_processing_control;
DROP FUNCTION advance_osrm_match_target();
DROP FUNCTION advance_chunked_osrm_match_target();
ALTER TABLE device_processing_control
    DROP COLUMN match_min_confidence,
    DROP COLUMN match_max_attempts,
    DROP COLUMN match_retry_delay_ms,
    DROP COLUMN match_total_budget_ms,
    DROP COLUMN matcher_engine_id,
    DROP COLUMN matcher_dataset_id,
    DROP COLUMN match_chunk_max_points,
    DROP COLUMN match_chunk_overlap_points,
    ALTER COLUMN target_id SET DEFAULT 'processed-gps-v1';
ALTER TABLE processing_storage_measurements DROP COLUMN matcher_evidence_bytes;
UPDATE device_processing_control
SET target_id='processed-gps-v1', target_generation=target_generation+1,
    work_generation=work_generation+1;
UPDATE processing_days SET state='queued';
INSERT INTO processing_jobs(device_id,state)
SELECT device_id,'queued' FROM device_processing_control
ON CONFLICT(device_id) DO UPDATE SET state='queued',lease_until=NULL,failure_message=NULL;
