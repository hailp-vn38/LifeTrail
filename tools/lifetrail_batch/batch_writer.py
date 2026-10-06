"""Write immutable deterministic gps/1 Batch artifacts and verify each manifest."""

import hashlib
import json
import uuid
from .inspection import inspect_batch
from .simulate import GeneratedBatch


def write_batches(records, output, seed, batch_records):
    if batch_records <= 0:
        raise ValueError("batch_records must be positive")
    output.mkdir(parents=True, exist_ok=True)
    batches = []
    for offset in range(0, len(records), batch_records):
        points = records[offset : offset + batch_records]
        body = b"".join(
            json.dumps(p, separators=(",", ":"), allow_nan=False).encode("ascii")
            + b"\n"
            for p in points
        )
        digest = hashlib.sha256(body).hexdigest()
        identity = hashlib.sha256(f"{seed}|{offset}|{digest}".encode()).digest()
        batch_id = str(uuid.UUID(bytes=identity[:16], version=4))
        body_path = output / f"{batch_id}.ndjson.ready"
        manifest_path = output / f"{batch_id}.manifest"
        if body_path.exists() and body_path.read_bytes() != body:
            raise ValueError("refusing to overwrite a different immutable Batch")
        body_path.write_bytes(body)
        manifest = {
            "batch_id": batch_id,
            "schema": "gps/1",
            "byte_length": len(body),
            "sha256": digest,
            "record_count": len(points),
            "first_ts_ms": points[0]["ts_ms"],
            "last_ts_ms": points[-1]["ts_ms"],
        }
        manifest_path.write_text(
            "".join(f"{k}={v}\n" for k, v in manifest.items()), encoding="ascii"
        )
        inspect_batch(body_path, manifest_path)
        batches.append(
            GeneratedBatch(
                batch_id,
                body_path,
                manifest_path,
                len(points),
                points[0]["ts_ms"],
                points[-1]["ts_ms"],
            )
        )
    return batches
