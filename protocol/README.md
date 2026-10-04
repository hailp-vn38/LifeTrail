# LifeTrail protocol contracts

These files are the canonical Phase 1 wire contracts shared by firmware, server and generated Web clients:

- [`gps-record-v1.md`](gps-record-v1.md): one `gps/1` record and its NDJSON framing.
- [`sync-batch-v1.md`](sync-batch-v1.md): durable Batch upload, verification, replay and retry semantics.
- [`openapi/lifetrail-v1.yaml`](openapi/lifetrail-v1.yaml): API-v1-facing reusable error envelope.
- [`fixtures/README.md`](fixtures/README.md): byte fixtures and expected results for independent validators.

An implementation may add a stricter internal representation, but cannot redefine these bytes or API-v1 semantics.
