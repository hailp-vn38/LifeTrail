#!/usr/bin/env python3
"""Generate deterministic, realistic gps/1 fixtures for LifeTrail.

Scenario: a person walks in central Ho Chi Minh City, pauses at two locations,
returns over an earlier path, walks the same city-block loop twice, then returns.
The static route anchors follow road corridors around Ben Thanh / Le Loi /
Nguyen Hue / Le Thanh Ton / Dong Khoi so the line is suitable for MapLibre UI
and playback tests without any online routing dependency.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import urllib.error
import urllib.request
import uuid
from dataclasses import dataclass
from datetime import datetime, time, timedelta, timezone
from pathlib import Path
from zoneinfo import ZoneInfo

SCHEMA = "gps/1"
DEFAULT_TIMEZONE = "Asia/Ho_Chi_Minh"
DEFAULT_DATE = "2026-10-05"
DEFAULT_START = time(8, 0, 0)
DEFAULT_BATCH_RECORDS = 300
SEED = "lifetrail-realistic-human-v1"

# Coordinates are (lat, lon). Dense anchors approximate road centerlines.
# A: Ben Thanh / Le Loi corridor -> B: Nguyen Hue / Le Loi.
LE_LOI_EAST = (
    (10.772750, 106.698120),
    (10.772970, 106.698480),
    (10.773120, 106.698880),
    (10.773300, 106.699330),
    (10.773480, 106.699780),
    (10.773670, 106.700250),
    (10.773850, 106.700710),
    (10.774020, 106.701170),
    (10.774170, 106.701630),
    (10.774280, 106.702080),
    (10.774370, 106.702520),
    (10.774430, 106.702960),
)

# B -> City Hall area northbound on Nguyen Hue.
NGUYEN_HUE_NORTH = (
    (10.774430, 106.702960),
    (10.774820, 106.703010),
    (10.775220, 106.703050),
    (10.775650, 106.703090),
    (10.776080, 106.703130),
    (10.776510, 106.703170),
    (10.776940, 106.703200),
    (10.777360, 106.703230),
    (10.777720, 106.703250),
)

# A rectangular-ish block loop: Nguyen Hue -> Le Thanh Ton -> Dong Khoi -> Le Loi.
CITY_BLOCK_LOOP = (
    (10.774430, 106.702960),
    (10.774850, 106.703010),
    (10.775300, 106.703060),
    (10.775760, 106.703100),
    (10.776210, 106.703150),
    (10.776670, 106.703190),
    (10.777050, 106.703220),
    (10.777100, 106.703610),
    (10.777110, 106.704030),
    (10.777100, 106.704450),
    (10.776660, 106.704470),
    (10.776190, 106.704460),
    (10.775730, 106.704450),
    (10.775270, 106.704430),
    (10.774820, 106.704400),
    (10.774430, 106.704360),
    (10.774420, 106.703980),
    (10.774420, 106.703620),
    (10.774430, 106.703280),
    (10.774430, 106.702960),
)


@dataclass(frozen=True)
class Segment:
    key: str
    kind: str  # "move" or "stop"
    duration_s: int
    label: str
    path: tuple[tuple[float, float], ...] | None = None
    center: tuple[float, float] | None = None


SCENARIO = (
    Segment("trip_01", "move", 570, "Ben Thanh -> Nguyen Hue", LE_LOI_EAST),
    Segment("stop_01", "stop", 480, "Coffee / stationary at Nguyen Hue", center=LE_LOI_EAST[-1]),
    Segment("trip_02", "move", 390, "Nguyen Hue -> City Hall", NGUYEN_HUE_NORTH),
    Segment("stop_02", "stop", 180, "Short stop near City Hall", center=NGUYEN_HUE_NORTH[-1]),
    Segment("trip_03", "move", 360, "Return to Nguyen Hue over same path", tuple(reversed(NGUYEN_HUE_NORTH))),
    Segment("stop_03", "stop", 75, "Brief wait before continuing", center=LE_LOI_EAST[-1]),
    Segment("trip_04", "move", 720, "City block loop - pass 1", CITY_BLOCK_LOOP),
    Segment("stop_04", "stop", 360, "Second coffee / stationary", center=LE_LOI_EAST[-1]),
    Segment("trip_05", "move", 630, "City block loop - repeated pass 2", CITY_BLOCK_LOOP),
    Segment("trip_06", "move", 540, "Nguyen Hue -> Ben Thanh", tuple(reversed(LE_LOI_EAST))),
    Segment("stop_05", "stop", 120, "End of walk / stationary", center=LE_LOI_EAST[0]),
)


def haversine_m(a: tuple[float, float], b: tuple[float, float]) -> float:
    r = 6_371_000.0
    p1 = math.radians(a[0])
    p2 = math.radians(b[0])
    dp = math.radians(b[0] - a[0])
    dl = math.radians(b[1] - a[1])
    h = math.sin(dp / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(dl / 2) ** 2
    return 2 * r * math.atan2(math.sqrt(h), math.sqrt(max(0.0, 1 - h)))


def bearing_deg(a: tuple[float, float], b: tuple[float, float]) -> float:
    if a == b:
        return 0.0
    p1 = math.radians(a[0])
    p2 = math.radians(b[0])
    dl = math.radians(b[1] - a[1])
    y = math.sin(dl) * math.cos(p2)
    x = math.cos(p1) * math.sin(p2) - math.sin(p1) * math.cos(p2) * math.cos(dl)
    return (math.degrees(math.atan2(y, x)) + 360.0) % 360.0


def path_lengths(path: tuple[tuple[float, float], ...]) -> tuple[list[float], float]:
    lengths = [haversine_m(path[i], path[i + 1]) for i in range(len(path) - 1)]
    return lengths, sum(lengths)


def point_along_path(path: tuple[tuple[float, float], ...], fraction: float) -> tuple[float, float]:
    lengths, total = path_lengths(path)
    if total <= 0:
        return path[0]
    target = max(0.0, min(1.0, fraction)) * total
    walked = 0.0
    for i, seg_len in enumerate(lengths):
        if target <= walked + seg_len or i == len(lengths) - 1:
            ratio = 0.0 if seg_len == 0 else (target - walked) / seg_len
            ratio = max(0.0, min(1.0, ratio))
            a, b = path[i], path[i + 1]
            return (a[0] + (b[0] - a[0]) * ratio, a[1] + (b[1] - a[1]) * ratio)
        walked += seg_len
    return path[-1]


def movement_progress(elapsed: int, duration: int, salt: int) -> float:
    """Variable human pace with two short crossing slow-downs, normalized to 0..1."""
    if duration <= 0:
        return 1.0
    n = duration
    # Deterministic integration. Duration is at most 720s, so this is cheap.
    weights = []
    for t in range(n):
        w = 1.0 + 0.18 * math.sin((t + 13 * salt) / 27.0) + 0.08 * math.sin((t + 7 * salt) / 8.5)
        # Simulate waiting/slowing at two crossings during each moving segment.
        p = t / n
        if 0.31 <= p <= 0.34 or 0.67 <= p <= 0.70:
            w *= 0.18
        weights.append(max(0.05, w))
    upto = min(max(elapsed, 0), n)
    return sum(weights[:upto]) / sum(weights)


def stop_point(center: tuple[float, float], elapsed: int, salt: int) -> tuple[float, float]:
    """Sub-2m correlated GNSS drift while stationary."""
    # East/north offsets in meters, deliberately slow-moving to avoid unrealistic zig-zag.
    east_m = 0.75 * math.sin((elapsed + salt * 17) / 61.0) + 0.22 * math.sin(elapsed / 17.0)
    north_m = 0.65 * math.cos((elapsed + salt * 11) / 73.0) + 0.18 * math.sin(elapsed / 23.0)
    dlat = north_m / 111_320.0
    dlon = east_m / (111_320.0 * math.cos(math.radians(center[0])))
    return center[0] + dlat, center[1] + dlon


def scenario_position(second: int) -> tuple[tuple[float, float], Segment, int, int]:
    cursor = 0
    for idx, seg in enumerate(SCENARIO):
        end = cursor + seg.duration_s
        if second < end or idx == len(SCENARIO) - 1:
            elapsed = max(0, min(second - cursor, seg.duration_s))
            if seg.kind == "move":
                assert seg.path is not None
                pos = point_along_path(seg.path, movement_progress(elapsed, seg.duration_s, idx + 1))
            else:
                assert seg.center is not None
                pos = stop_point(seg.center, elapsed, idx + 1)
            return pos, seg, idx, elapsed
        cursor = end
    last = SCENARIO[-1]
    assert last.center is not None
    return last.center, last, len(SCENARIO) - 1, last.duration_s


def local_start(date: str, timezone_name: str) -> datetime:
    d = datetime.strptime(date, "%Y-%m-%d").date()
    return datetime.combine(d, DEFAULT_START, tzinfo=ZoneInfo(timezone_name))


def build_records(date: str, timezone_name: str) -> tuple[list[dict[str, object]], datetime]:
    start = local_start(date, timezone_name)
    total_seconds = sum(s.duration_s for s in SCENARIO)
    points = [scenario_position(t)[0] for t in range(total_seconds + 1)]
    records: list[dict[str, object]] = []
    for i, point in enumerate(points):
        ts = start + timedelta(seconds=i)
        if i < len(points) - 1:
            next_point = points[i + 1]
            speed = haversine_m(point, next_point)
            course = bearing_deg(point, next_point) if speed >= 0.20 else None
        else:
            speed = haversine_m(points[i - 1], point) if i else 0.0
            course = None

        _, seg, seg_idx, _ = scenario_position(i)
        highrise_factor = 0.22 if seg.key in {"trip_04", "trip_05"} else 0.0
        hdop = 0.82 + highrise_factor + 0.16 * (1 + math.sin(i / 47.0)) / 2
        satellites = 12 + int(round(2 * math.sin(i / 71.0)))
        if seg.kind == "stop":
            speed = min(speed, 0.08)
            course = None

        records.append(
            {
                "ts_ms": int(ts.timestamp() * 1000),
                "lat": round(point[0], 6),
                "lon": round(point[1], 6),
                "alt_m": round(7.4 + 0.55 * math.sin(i / 211.0) + 0.15 * math.sin(i / 31.0), 1),
                "speed_mps": round(speed, 2),
                "course_deg": None if course is None else round(course, 1),
                "fix_quality": 1,
                "satellites": max(8, min(16, satellites)),
                "hdop": round(hdop, 2),
            }
        )
    return records, start


def deterministic_uuid(date: str, timezone_name: str, batch_index: int) -> str:
    digest = hashlib.sha256(f"{SEED}|{date}|{timezone_name}|{batch_index}".encode()).digest()
    return str(uuid.UUID(bytes=digest[:16], version=4))


def write_manifest(path: Path, batch_id: str, body: bytes, records: list[dict[str, object]]) -> None:
    sha = hashlib.sha256(body).hexdigest()
    path.write_text(
        "\n".join(
            [
                f"batch_id={batch_id}",
                f"schema={SCHEMA}",
                f"byte_length={len(body)}",
                f"sha256={sha}",
                f"record_count={len(records)}",
                f"first_ts_ms={records[0]['ts_ms']}",
                f"last_ts_ms={records[-1]['ts_ms']}",
            ]
        )
        + "\n",
        encoding="ascii",
    )


def parse_manifest(path: Path) -> dict[str, str]:
    out: dict[str, str] = {}
    for line in path.read_text(encoding="ascii").splitlines():
        k, v = line.split("=", 1)
        out[k] = v
    return out


def upload_batch(endpoint: str, token: str, body_path: Path, manifest_path: Path) -> dict[str, object]:
    manifest = parse_manifest(manifest_path)
    body = body_path.read_bytes()
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/x-ndjson",
        "X-LifeTrail-Batch-Id": manifest["batch_id"],
        "X-LifeTrail-Schema": SCHEMA,
        "X-LifeTrail-Content-SHA256": manifest["sha256"],
        "X-LifeTrail-Byte-Length": manifest["byte_length"],
        "X-LifeTrail-Record-Count": manifest["record_count"],
    }
    req = urllib.request.Request(endpoint, data=body, headers=headers, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=20) as resp:
            payload = json.loads(resp.read())
    except urllib.error.HTTPError as exc:
        details = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"upload failed HTTP {exc.code}: {details}") from exc
    if not isinstance(payload, dict):
        raise RuntimeError("server returned non-object JSON")
    return payload


def build_timeline(start: datetime) -> dict[str, object]:
    cursor = 0
    trips = []
    stops = []
    events = []
    for idx, seg in enumerate(SCENARIO):
        seg_start = start + timedelta(seconds=cursor)
        seg_end = seg_start + timedelta(seconds=seg.duration_s)
        pos_start = scenario_position(cursor)[0]
        pos_end = scenario_position(min(cursor + seg.duration_s, sum(s.duration_s for s in SCENARIO)))[0]
        item_id = str(uuid.uuid5(uuid.NAMESPACE_URL, f"{SEED}:{seg.key}"))
        if seg.kind == "move":
            assert seg.path is not None
            _, distance = path_lengths(seg.path)
            trips.append(
                {
                    "id": item_id,
                    "segment_key": seg.key,
                    "start_at": seg_start.astimezone(timezone.utc).isoformat().replace("+00:00", "Z"),
                    "end_at": seg_end.astimezone(timezone.utc).isoformat().replace("+00:00", "Z"),
                    "duration_s": seg.duration_s,
                    "expected_distance_m": round(distance, 1),
                    "label": seg.label,
                }
            )
            for event_type, when, pos in (
                ("TRIP_START", seg_start, pos_start),
                ("TRIP_END", seg_end, pos_end),
            ):
                events.append(
                    {
                        "id": str(uuid.uuid5(uuid.NAMESPACE_URL, f"{SEED}:{seg.key}:{event_type}")),
                        "event_type": event_type,
                        "occurred_at": when.astimezone(timezone.utc).isoformat().replace("+00:00", "Z"),
                        "location": {"type": "Point", "coordinates": [round(pos[1], 6), round(pos[0], 6)]},
                        "metadata": {"segment_key": seg.key, "label": seg.label},
                        "source_type": "simulator",
                        "source_id": item_id,
                        "algorithm_version": "fixture-ground-truth-v1",
                    }
                )
        else:
            assert seg.center is not None
            stops.append(
                {
                    "id": item_id,
                    "segment_key": seg.key,
                    "start_at": seg_start.astimezone(timezone.utc).isoformat().replace("+00:00", "Z"),
                    "end_at": seg_end.astimezone(timezone.utc).isoformat().replace("+00:00", "Z"),
                    "duration_s": seg.duration_s,
                    "center": {"type": "Point", "coordinates": [seg.center[1], seg.center[0]]},
                    "radius_m": 2.0,
                    "label": seg.label,
                }
            )
            events.append(
                {
                    "id": str(uuid.uuid5(uuid.NAMESPACE_URL, f"{SEED}:{seg.key}:STOP")),
                    "event_type": "STOP",
                    "occurred_at": seg_start.astimezone(timezone.utc).isoformat().replace("+00:00", "Z"),
                    "location": {"type": "Point", "coordinates": [seg.center[1], seg.center[0]]},
                    "metadata": {"segment_key": seg.key, "duration_s": seg.duration_s, "label": seg.label},
                    "source_type": "simulator",
                    "source_id": item_id,
                    "algorithm_version": "fixture-ground-truth-v1",
                }
            )
        cursor += seg.duration_s
    return {
        "schema": "lifetrail/simulated-timeline/1",
        "note": "Ground truth sidecar. Phase 1 server ingests only gps/1; trips/stops/timeline are future derived data.",
        "trips": trips,
        "stops": stops,
        "timeline_events": sorted(events, key=lambda e: e["occurred_at"]),
    }


def build_daily_view(records: list[dict[str, object]], date: str, timezone_name: str) -> dict[str, object]:
    coords = [[r["lon"], r["lat"]] for r in records]
    timestamps = [datetime.fromtimestamp(r["ts_ms"] / 1000, tz=timezone.utc).isoformat().replace("+00:00", "Z") for r in records]
    distance = sum(haversine_m((records[i]["lat"], records[i]["lon"]), (records[i + 1]["lat"], records[i + 1]["lon"])) for i in range(len(records) - 1))
    feature = {
        "type": "Feature",
        "properties": {"timestamps": timestamps},
        "geometry": {"type": "LineString", "coordinates": coords},
    }
    return {
        "device_id": "00000000-0000-0000-0000-000000000001",
        "date": date,
        "timezone": timezone_name,
        "processing_state": "raw",
        "summary": {
            "point_count": len(records),
            "distance_m": distance,
            "duration_s": int((records[-1]["ts_ms"] - records[0]["ts_ms"]) / 1000),
            "first_fix_at": timestamps[0],
            "last_fix_at": timestamps[-1],
        },
        "route": feature,
        "start": {"type": "Feature", "properties": {"recorded_at": timestamps[0]}, "geometry": {"type": "Point", "coordinates": coords[0]}},
        "end": {"type": "Feature", "properties": {"recorded_at": timestamps[-1]}, "geometry": {"type": "Point", "coordinates": coords[-1]}},
    }


def generate(output: Path, date: str, timezone_name: str, batch_records: int) -> dict[str, object]:
    output.mkdir(parents=True, exist_ok=True)
    batches_dir = output / "batches"
    batches_dir.mkdir(exist_ok=True)
    records, start = build_records(date, timezone_name)

    batch_entries = []
    for batch_idx, offset in enumerate(range(0, len(records), batch_records)):
        batch = records[offset : offset + batch_records]
        batch_id = deterministic_uuid(date, timezone_name, batch_idx)
        body = b"".join(json.dumps(r, separators=(",", ":"), ensure_ascii=True).encode("ascii") + b"\n" for r in batch)
        body_path = batches_dir / f"{batch_id}.ndjson.ready"
        manifest_path = batches_dir / f"{batch_id}.manifest"
        body_path.write_bytes(body)
        write_manifest(manifest_path, batch_id, body, batch)
        batch_entries.append({
            "batch_id": batch_id,
            "body": str(body_path.relative_to(output)),
            "manifest": str(manifest_path.relative_to(output)),
            "record_count": len(batch),
            "byte_length": len(body),
        })

    timeline = build_timeline(start)
    (output / "timeline.mock.json").write_text(json.dumps(timeline, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    daily_view = build_daily_view(records, date, timezone_name)
    (output / "daily-view.mock.json").write_text(json.dumps(daily_view, separators=(",", ":"), ensure_ascii=False) + "\n", encoding="utf-8")

    preview = {
        "type": "FeatureCollection",
        "features": [daily_view["route"]]
        + [
            {
                "type": "Feature",
                "properties": {"kind": "stop", "segment_key": s["segment_key"], "label": s["label"], "duration_s": s["duration_s"]},
                "geometry": s["center"],
            }
            for s in timeline["stops"]
        ],
    }
    (output / "route-preview.geojson").write_text(json.dumps(preview, separators=(",", ":"), ensure_ascii=False) + "\n", encoding="utf-8")

    moving_s = sum(s.duration_s for s in SCENARIO if s.kind == "move")
    stopped_s = sum(s.duration_s for s in SCENARIO if s.kind == "stop")
    summary = {
        "schema": SCHEMA,
        "date": date,
        "timezone": timezone_name,
        "start_local": start.isoformat(),
        "end_local": (start + timedelta(seconds=moving_s + stopped_s)).isoformat(),
        "duration_s": moving_s + stopped_s,
        "moving_s": moving_s,
        "stopped_s": stopped_s,
        "record_count": len(records),
        "batch_count": len(batch_entries),
        "raw_daily_distance_m": round(daily_view["summary"]["distance_m"], 1),
        "ground_truth_trip_distance_m": round(sum(t["expected_distance_m"] for t in timeline["trips"]), 1),
        "batches": batch_entries,
    }
    (output / "summary.json").write_text(json.dumps(summary, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return summary


def upload_all(output: Path, endpoint: str, token: str) -> list[dict[str, object]]:
    summary = json.loads((output / "summary.json").read_text(encoding="utf-8"))
    responses = []
    for item in summary["batches"]:
        responses.append(upload_batch(endpoint, token, output / item["body"], output / item["manifest"]))
    return responses


def main() -> int:
    ap = argparse.ArgumentParser(description="Generate realistic LifeTrail gps/1 + timeline ground-truth fixtures")
    ap.add_argument("--date", default=DEFAULT_DATE)
    ap.add_argument("--timezone", default=DEFAULT_TIMEZONE)
    ap.add_argument("--output", type=Path, default=Path("tools/fixtures/realistic-human-day"))
    ap.add_argument("--batch-records", type=int, default=DEFAULT_BATCH_RECORDS)
    ap.add_argument("--endpoint")
    ap.add_argument("--token")
    args = ap.parse_args()
    if bool(args.endpoint) != bool(args.token):
        ap.error("--endpoint and --token must be supplied together")
    summary = generate(args.output, args.date, args.timezone, args.batch_records)
    if args.endpoint:
        summary["upload_responses"] = upload_all(args.output, args.endpoint, args.token)
    print(json.dumps(summary, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
