-- Range publication bookkeeping.  Raw GPS remains immutable; these bounds say
-- which portion of the published interpretation must be refreshed.
ALTER TABLE device_processing_control
    ADD COLUMN dirty_from_at TIMESTAMPTZ,
    ADD COLUMN dirty_until_at TIMESTAMPTZ,
    ADD COLUMN active_manifest_id UUID;

ALTER TABLE activity_revisions
    ADD COLUMN supersedes_from_at TIMESTAMPTZ,
    ADD COLUMN supersedes_until_at TIMESTAMPTZ,
    ADD CHECK ((supersedes_from_at IS NULL) = (supersedes_until_at IS NULL)),
    ADD CHECK (supersedes_until_at IS NULL OR supersedes_until_at > supersedes_from_at);

-- `active_manifest_id` deliberately has no foreign key: activity manifests are
-- immutable history and this pointer is the one mutable authority.  A
-- candidate is inserted before the short activation transaction.
CREATE INDEX activity_revisions_device_range
    ON activity_revisions(device_id, observed_from_at, observed_until_at);
