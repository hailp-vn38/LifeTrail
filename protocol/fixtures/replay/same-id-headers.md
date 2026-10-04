# Same batch ID, different body

Apply the following request metadata to both bodies. The byte length and SHA-256 value must be calculated from the named file bytes, including its final LF.

```http
X-LifeTrail-Batch-Id: 550e8400-e29b-41d4-a716-446655440000
X-LifeTrail-Schema: gps/1
X-LifeTrail-Record-Count: 1
```

Send `same-id-body-a.ndjson` first with its calculated integrity headers; it commits. Send `same-id-body-b.ndjson` with the same Batch ID but its own calculated integrity headers; it must return `409 batch_conflict`, even though its standalone payload is structurally valid.
