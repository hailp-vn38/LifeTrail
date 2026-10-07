"""Adaptive simulation batches using firmware policy decisions."""

from .batch_writer import encode_records
from .firmware_policy import Motion


def partition_accepted(accepted: list[dict], max_bytes: int = 262144) -> list[list[dict]]:
    groups = []
    current = []
    byte_length = 0
    for event in accepted:
        record = event["record"]
        age_ms = 300000 if event["motion"] == Motion.STATIONARY else 60000
        length = len(encode_records([record]))
        if length > max_bytes:
            raise ValueError("record exceeds batch maximum bytes")
        if current and (record["ts_ms"] - current[0]["ts_ms"] >= age_ms
                        or byte_length + length > max_bytes):
            groups.append(current)
            current = []
            byte_length = 0
        current.append(record)
        byte_length += length
    if current:
        groups.append(current)
    return groups
