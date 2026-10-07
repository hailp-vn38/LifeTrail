"""Apply the compiled Device persistence policy to actual simulated observations."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import tempfile
from enum import IntEnum

ROOT = Path(__file__).resolve().parents[2]


class Motion(IntEnum):
    """Host-filter adapter values from lifetrail_gps_policy.h."""

    MOVING = 0
    CANDIDATE_STOP = 1
    STATIONARY = 2


class PersistReason(IntEnum):
    PERIODIC = 0
    DISTANCE = 1
    HEADING = 2
    STATE_TRANSITION = 3
    BACKFILL = 4
    HEARTBEAT = 5
    SESSION_BOUNDARY = 6
    FIX_RECOVERY = 7


def policy_binary() -> Path:
    override = os.environ.get("LT_GPS_POLICY_FILTER")
    if override:
        return Path(override)
    build = Path(tempfile.gettempdir()) / "lifetrail-gps-policy-host"
    subprocess.run(
        ["cmake", "-S", str(ROOT / "firmware/esp32/host_tests"), "-B", str(build)],
        check=True, capture_output=True,
    )
    subprocess.run(
        ["cmake", "--build", str(build), "--target", "gps_policy_filter"],
        check=True, capture_output=True,
    )
    return build / "gps_policy_filter"


def filter_observations(records: list[dict], binary: Path | None = None) -> tuple[list[dict], dict]:
    """Select source records by timestamp; never duplicate the firmware algorithm."""
    def field(record: dict, name: str) -> str:
        value = record.get(name)
        return "nan" if value is None else str(value)

    fields = ("ts_ms", "lat", "lon", "speed_mps", "course_deg", "alt_m", "hdop", "fix_quality", "satellites")
    body = "".join(",".join(field(record, name) for name in fields) + "\n" for record in records)
    result = subprocess.run([str(binary or policy_binary())], input=body, text=True,
                            capture_output=True, check=True)
    by_timestamp = {int(record["ts_ms"]): record for record in records}
    accepted = []
    for line in result.stdout.splitlines():
        timestamp, motion, reason = map(int, line.split(","))
        accepted.append({"record": by_timestamp[timestamp],
                         "motion": Motion(motion), "reason": PersistReason(reason)})
    return accepted, json.loads(result.stderr)
