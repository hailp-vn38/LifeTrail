# Protocol fixtures

Fixtures are raw bytes, not examples to be reserialized. Validators on firmware and server sides must load the file bytes directly, apply the named header metadata where supplied, and compare against the expected result.

| Fixture | Expected result |
| --- | --- |
| `valid/final-lf.ndjson` | Valid UTF-8/LF `gps/1` body with two strictly increasing records. |
| `invalid/crlf.ndjson` | Reject: CRLF is forbidden. |
| `invalid/blank-line.ndjson` | Reject: blank physical line. |
| `invalid/missing-final-lf.ndjson` | Reject: final LF is required. |
| `invalid/malformed-json.ndjson` | Reject: malformed JSON. |
| `invalid/out-of-order-ts.ndjson` | Reject whole Batch: `ts_ms` is not strictly increasing. |
| `invalid/bad-range.ndjson` | Reject whole Batch: latitude is outside `[-90, 90]`. |
| `replay/same-id-body-a.ndjson` and `replay/same-id-body-b.ndjson` | With the shared batch ID in `same-id-headers.md`, body A commits first and body B must return `409 batch_conflict`. |

Every `.ndjson` fixture is intentionally small so it can be loaded by ESP-IDF and host/server test harnesses without a parser or SDK dependency. Line ending verification must happen before JSON parsing.
