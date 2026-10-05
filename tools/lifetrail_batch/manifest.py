import re
from pathlib import Path


MANIFEST_FIELDS = (
    "batch_id", "schema", "byte_length", "sha256", "record_count", "first_ts_ms", "last_ts_ms"
)
UUID_V4 = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


def read_manifest(path: Path) -> dict[str, object]:
    values: dict[str, str] = {}
    for line in path.read_text(encoding="ascii").splitlines():
        if "=" not in line:
            raise ValueError("manifest has a malformed line")
        name, value = line.split("=", 1)
        if name in values:
            raise ValueError(f"manifest repeats {name}")
        values[name] = value
    if set(values) != set(MANIFEST_FIELDS):
        raise ValueError("manifest fields differ from sync/1")
    if values["schema"] != "gps/1":
        raise ValueError("manifest schema must be gps/1")
    if not UUID_V4.fullmatch(values["batch_id"]):
        raise ValueError("manifest batch_id must be a canonical UUIDv4")
    if not SHA256.fullmatch(values["sha256"]):
        raise ValueError("manifest sha256 must be lowercase hex")
    try:
        numeric = {name: int(values[name]) for name in MANIFEST_FIELDS[2:] if name != "sha256"}
    except ValueError as error:
        raise ValueError("manifest numeric field is invalid") from error
    if any(value < 0 for value in numeric.values()) or numeric["record_count"] == 0:
        raise ValueError("manifest numeric field is out of range")
    return {"batch_id": values["batch_id"], "schema": values["schema"], "sha256": values["sha256"], **numeric}
