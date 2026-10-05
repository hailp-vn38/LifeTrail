import json


def parse_ndjson(body: bytes) -> list[dict[str, object]]:
    if not body:
        raise ValueError("NDJSON body is empty")
    if b"\r" in body:
        raise ValueError("NDJSON body contains CRLF or CR")
    if not body.endswith(b"\n"):
        raise ValueError("NDJSON body must end in LF")
    records = []
    last_timestamp = None
    for line_number, line in enumerate(body[:-1].split(b"\n"), start=1):
        if not line:
            raise ValueError(f"NDJSON line {line_number} is blank")
        try:
            record = json.loads(line)
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise ValueError(f"NDJSON line {line_number} is not a JSON object") from error
        if not isinstance(record, dict):
            raise ValueError(f"NDJSON line {line_number} is not a JSON object")
        timestamp = record.get("ts_ms")
        if not isinstance(timestamp, int) or isinstance(timestamp, bool) or timestamp <= 0:
            raise ValueError(f"NDJSON line {line_number} has invalid ts_ms")
        if last_timestamp is not None and timestamp <= last_timestamp:
            raise ValueError(f"NDJSON line {line_number} is not strictly increasing")
        records.append(record)
        last_timestamp = timestamp
    return records
