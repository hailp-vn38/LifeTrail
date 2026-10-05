import hashlib
from pathlib import Path

from .manifest import read_manifest
from .ndjson import parse_ndjson


def inspect_batch(body_path: Path, manifest_path: Path) -> dict[str, object]:
    """Verify a ready Batch against its manifest without changing either file."""
    body = body_path.read_bytes()
    manifest = read_manifest(manifest_path)
    records = parse_ndjson(body)
    observed = {
        "byte_length": len(body),
        "sha256": hashlib.sha256(body).hexdigest(),
        "record_count": len(records),
        "first_ts_ms": records[0]["ts_ms"],
        "last_ts_ms": records[-1]["ts_ms"],
    }
    for name, actual in observed.items():
        expected = manifest[name]
        if actual != expected:
            raise ValueError(f"manifest {name} mismatch: expected {expected!r}, got {actual!r}")
    return {**manifest, **observed}
