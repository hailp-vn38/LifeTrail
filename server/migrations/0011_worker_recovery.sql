-- A processing attempt is an auditable lease, not process-local state.  The
-- control row remains the per-Device serialization point; this table retains
-- each authority hand-off so an operator can distinguish a crash, rejection
-- and successful publication without deleting candidate history.
ALTER TABLE processing_jobs
    ADD COLUMN attempt_count BIGINT NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    ADD COLUMN claimed_at TIMESTAMPTZ,
    ADD COLUMN claimed_work_generation BIGINT,
    ADD COLUMN claimed_input_generation BIGINT,
    ADD COLUMN claimed_target_generation BIGINT,
    ADD COLUMN claimed_target_id TEXT;

CREATE TABLE processing_attempts (
    device_id UUID NOT NULL REFERENCES device_processing_control(device_id),
    fencing_token BIGINT NOT NULL,
    attempt BIGINT NOT NULL,
    work_generation BIGINT NOT NULL,
    input_generation BIGINT NOT NULL,
    target_generation BIGINT NOT NULL,
    target_id TEXT NOT NULL,
    claimed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    renewed_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    outcome TEXT CHECK (outcome IN ('activated', 'requeued', 'failed', 'superseded')),
    detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    PRIMARY KEY (device_id, fencing_token)
);

CREATE INDEX processing_attempts_device_claimed_at
    ON processing_attempts(device_id, claimed_at DESC);

-- Retain compact storage observations alongside immutable artifacts.  There
-- is intentionally no delete path: Phase 2 retention is reference-aware.
CREATE TABLE processing_storage_measurements (
    id UUID PRIMARY KEY,
    device_id UUID NOT NULL REFERENCES devices(id),
    fencing_token BIGINT NOT NULL,
    activity_revision_bytes BIGINT,
    manifest_count BIGINT NOT NULL,
    snapshot_bytes BIGINT,
    candidate_bytes BIGINT,
    matcher_evidence_bytes BIGINT,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
