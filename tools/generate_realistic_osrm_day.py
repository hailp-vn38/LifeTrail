#!/usr/bin/env python3
"""Create and optionally upload a realistic, seeded OSRM scenario day."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from lifetrail_batch.realistic import generate_realistic_day
from lifetrail_batch.simulate import upload_route


def main():
    parser = argparse.ArgumentParser(description="Generate realistic road-network gps/1 Batches.")
    parser.add_argument("--scenario", required=True, type=Path)
    parser.add_argument("--routing-versions", required=True, type=Path)
    parser.add_argument("--osrm-urls", required=True, type=Path, help="JSON object keyed by car, bike and foot")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--batch-records", type=int, default=300)
    parser.add_argument("--endpoint")
    parser.add_argument("--token")
    args = parser.parse_args()
    if bool(args.endpoint) != bool(args.token):
        parser.error("--endpoint and --token must be supplied together")
    try:
        batches = generate_realistic_day(
            json.loads(args.scenario.read_text()), json.loads(args.osrm_urls.read_text()),
            json.loads(args.routing_versions.read_text()), args.output, batch_records=args.batch_records,
        )
        result = {"batch_ids": [batch.batch_id for batch in batches], "record_count": sum(batch.record_count for batch in batches)}
        if args.endpoint:
            result["commits"] = upload_route(args.endpoint, args.token, batches)
            result["replays"] = upload_route(args.endpoint, args.token, batches)
        print(json.dumps(result, indent=2, sort_keys=True))
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        parser.exit(1, f"realistic scenario failed: {error}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
