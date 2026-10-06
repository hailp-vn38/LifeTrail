-- Operator requests change pending work without changing immutable Raw input.
ALTER TABLE device_processing_control
ADD COLUMN work_generation BIGINT NOT NULL DEFAULT 0 CHECK (work_generation >= 0);
