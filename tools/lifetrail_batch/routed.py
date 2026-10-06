"""Operator-facing scenario generation; no activity inference happens here."""

import argparse
import json
from datetime import datetime
from pathlib import Path
from zoneinfo import ZoneInfo
from . import osrm_client
from .batch_writer import write_batches
from .route_sampling import sample, validate_coordinates
from .simulate import upload_route


def generate_scenario(scenario, base_url, metadata, output, *, batch_records=300):
    profile = scenario["profile"]
    if profile not in osrm_client.SERVICE_PROFILE or metadata.get("profile") != profile:
        raise ValueError(
            "scenario profile and dataset metadata must match car/bike/foot"
        )
    if not metadata.get("engine_version") or not metadata.get("dataset_version"):
        raise ValueError("dataset/engine provenance is required")
    duration = scenario["duration_s"]
    interval = scenario.get("interval_s", 1)
    if any(
        not isinstance(v, int) or isinstance(v, bool) or v <= 0
        for v in (duration, interval, batch_records)
    ):
        raise ValueError("duration, interval and batch size must be positive integers")
    start = datetime.fromisoformat(scenario["started_at"])
    if start.tzinfo is None:
        raise ValueError("started_at must have an explicit UTC offset")
    zone = ZoneInfo(scenario["timezone"])
    if start.utcoffset() != start.astimezone(zone).utcoffset():
        raise ValueError("started_at offset must match scenario timezone")
    validate_coordinates(scenario["waypoints"])
    coordinates, evidence = osrm_client.route(base_url, profile, scenario["waypoints"])
    records = sample(
        coordinates, int(start.timestamp() * 1000), duration, interval, scenario["seed"]
    )
    batches = write_batches(records, output, scenario["seed"], batch_records)
    (output / "route-evidence.json").write_text(
        json.dumps(
            {
                **metadata,
                **evidence,
                "scenario": scenario,
                "batch_ids": [b.batch_id for b in batches],
            },
            indent=2,
            sort_keys=True,
        )
        + "\n"
    )
    return batches


def main():
    parser = argparse.ArgumentParser(
        description="Generate road-routed gps/1 fixtures with recorded provenance."
    )
    parser.add_argument("--scenario", required=True, type=Path)
    parser.add_argument("--metadata", required=True, type=Path)
    parser.add_argument("--osrm-url", required=True)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--batch-records", type=int, default=300)
    parser.add_argument("--endpoint")
    parser.add_argument("--token")
    args = parser.parse_args()
    if bool(args.endpoint) != bool(args.token):
        parser.error("--endpoint and --token must be supplied together")
    try:
        scenario = json.loads(args.scenario.read_text())
        batches = generate_scenario(
            scenario,
            args.osrm_url,
            json.loads(args.metadata.read_text()),
            args.output,
            batch_records=args.batch_records,
        )
        result = {
            "batch_ids": [b.batch_id for b in batches],
            "record_count": sum(b.record_count for b in batches),
        }
        if args.endpoint:
            result["commits"] = upload_route(args.endpoint, args.token, batches)
            result["replays"] = upload_route(args.endpoint, args.token, batches)
        print(json.dumps(result, indent=2))
    except (OSError, ValueError, KeyError) as error:
        parser.exit(1, f"routed scenario failed: {error}\n")
    return 0
