"""Write immutable gps/1 Batch artifacts shared by route generators."""

import hashlib
import json
import uuid
from dataclasses import dataclass
from pathlib import Path
from .inspection import inspect_batch


@dataclass(frozen=True)
class GeneratedBatch:
    batch_id: str
    body_path: Path
    manifest_path: Path
    record_count: int
    first_ts_ms: int
    last_ts_ms: int


def encode_records(records):
    return b"".join(
        json.dumps(p, separators=(",", ":"), allow_nan=False).encode("ascii") + b"\n"
        for p in records
    )


def write_batch(output, batch_id, points, body):
    output.mkdir(parents=True, exist_ok=True)
    body_path = output / f"{batch_id}.ndjson.ready"
    manifest_path = output / f"{batch_id}.manifest"
    if body_path.exists() and body_path.read_bytes() != body:
        raise ValueError("refusing to overwrite a different immutable Batch")
    body_path.write_bytes(body)
    manifest = {
        "batch_id": batch_id,
        "schema": "gps/1",
        "byte_length": len(body),
        "sha256": hashlib.sha256(body).hexdigest(),
        "record_count": len(points),
        "first_ts_ms": points[0]["ts_ms"],
        "last_ts_ms": points[-1]["ts_ms"],
    }
    manifest_path.write_text(
        "".join(f"{k}={v}\n" for k, v in manifest.items()), encoding="ascii"
    )
    inspect_batch(body_path, manifest_path)
    return GeneratedBatch(
        batch_id,
        body_path,
        manifest_path,
        len(points),
        points[0]["ts_ms"],
        points[-1]["ts_ms"],
    )


def write_batches(records, output, seed, batch_records):
    if batch_records <= 0:
        raise ValueError("batch_records must be positive")
    batches = []
    for offset in range(0, len(records), batch_records):
        points = records[offset : offset + batch_records]
        body = encode_records(points)
        digest = hashlib.sha256(body).hexdigest()
        identity = hashlib.sha256(f"{seed}|{offset}|{digest}".encode()).digest()
        batch_id = str(uuid.UUID(bytes=identity[:16], version=4))
        batches.append(write_batch(output, batch_id, points, body))
    return batches
