-- Movement between Stops becomes a Trip of ordered UNKNOWN Movement Segments.
-- The processing target identity changes with the reducer, so publication
-- freshness reports existing snapshots as stale until they are rebuilt.
ALTER TABLE device_processing_control ALTER COLUMN target_id SET DEFAULT 'trips-v1';
UPDATE device_processing_control SET target_id='trips-v1', target_generation=target_generation+1;
