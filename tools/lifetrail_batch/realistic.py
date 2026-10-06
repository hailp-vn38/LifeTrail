"""Generate inspectable multi-mode road-network days without activity inference."""

from __future__ import annotations

import json
from datetime import datetime
from pathlib import Path
from zoneinfo import ZoneInfo

from . import osrm_client
from .batch_writer import write_batches
from .realistic_sampling import degrade, moving_records, stopped_records
from .route_sampling import validate_coordinates

VALID_MODES = frozenset(osrm_client.SERVICE_PROFILE)
VALID_DEGRADATIONS = frozenset({"poor_quality", "impossible_jump", "gnss_loss"})


def generate_realistic_day(scenario, osrm_urls, routing_versions, output, *, batch_records=300):
    """Create normal Batch artifacts plus factual scenario and Route evidence sidecars."""
    _validate(scenario, osrm_urls, routing_versions, batch_records)
    output.mkdir(parents=True, exist_ok=True)
    started_at = datetime.fromisoformat(scenario["started_at"])
    start_ms = int(started_at.timestamp() * 1000)
    records, truth, routes = _generate_records(scenario, osrm_urls, routing_versions, start_ms)
    batches = write_batches(records, output, str(scenario["seed"]), batch_records)
    _write_json(output / "scenario.json", scenario)
    _write_json(output / "ground-truth.json", {"scenario_id": scenario["id"], "seed": scenario["seed"], "activities": truth["activities"], "degradations": truth["degradations"]})
    _write_json(output / "routing-evidence.json", {"scenario_id": scenario["id"], "seed": scenario["seed"], "routes": routes, "batch_ids": [batch.batch_id for batch in batches]})
    return batches


def _generate_records(scenario, urls, versions, start_ms):
    current_ms = start_ms
    prior_timestamp = None
    all_records, activities, degradations, routes = [], [], [], []
    for activity_index, activity in enumerate(scenario["activities"]):
        duration = activity["duration_s"]
        key = f"{scenario['seed']}:{activity_index}"
        if activity["kind"] == "move":
            profile = activity["mode"]
            coordinates, evidence = osrm_client.route(urls[profile], profile, [scenario["places"][activity["from"]], scenario["places"][activity["to"]]])
            raw_records = moving_records(coordinates, current_ms, duration, scenario["interval_s"], key)
            routes.append({"activity_index": activity_index, "profile": profile, **versions[profile], **evidence})
        else:
            raw_records = stopped_records(scenario["places"][activity["at"]], current_ms, duration, scenario["interval_s"], key)
        activity_degradations = activity.get("degradations", [])
        raw_records = degrade(raw_records, activity_degradations, scenario["interval_s"])
        # Adjacent activities share their boundary; one Raw timestamp must occur once.
        raw_records = [record for record in raw_records if prior_timestamp is None or record["ts_ms"] > prior_timestamp]
        if raw_records:
            prior_timestamp = raw_records[-1]["ts_ms"]
            all_records.extend(raw_records)
        activities.append(_activity_truth(activity, activity_index, current_ms, duration))
        degradations.extend(_degradation_truth(activity_degradations, activity_index, current_ms))
        current_ms += duration * 1000
    if not all_records:
        raise ValueError("scenario contains no observable GPS records")
    return all_records, {"activities": activities, "degradations": degradations}, routes


def _validate(scenario, urls, versions, batch_records):
    required = {"id", "timezone", "seed", "started_at", "interval_s", "places", "activities"}
    if not isinstance(scenario, dict) or required - scenario.keys():
        raise ValueError("scenario is missing required day fields")
    if not isinstance(scenario["interval_s"], int) or scenario["interval_s"] <= 0 or not isinstance(batch_records, int) or batch_records <= 0:
        raise ValueError("interval and batch size must be positive integers")
    started_at = datetime.fromisoformat(scenario["started_at"])
    if started_at.tzinfo is None or started_at.utcoffset() != started_at.astimezone(ZoneInfo(scenario["timezone"])).utcoffset():
        raise ValueError("started_at must include an offset matching timezone")
    if not isinstance(scenario["places"], dict) or not scenario["places"]:
        raise ValueError("scenario needs named places")
    for coordinate in scenario["places"].values():
        validate_coordinates([coordinate, coordinate])
    if not isinstance(scenario["activities"], list) or not scenario["activities"]:
        raise ValueError("scenario needs activities")
    for activity in scenario["activities"]:
        if activity.get("kind") not in {"move", "stop"} or not isinstance(activity.get("duration_s"), int) or activity["duration_s"] <= 0:
            raise ValueError("each activity needs a kind and positive duration")
        if activity["kind"] == "move":
            if activity.get("mode") not in VALID_MODES or activity.get("from") not in scenario["places"] or activity.get("to") not in scenario["places"]:
                raise ValueError("movement needs valid mode and named endpoints")
            _validate_routing(activity["mode"], urls, versions)
        elif activity.get("at") not in scenario["places"]:
            raise ValueError("stop needs a named place")
        for degradation in activity.get("degradations", []):
            _validate_degradation(degradation, activity["duration_s"])


def _validate_routing(mode, urls, versions):
    provenance = versions.get(mode, {})
    if not urls.get(mode) or not provenance.get("engine_version") or not provenance.get("dataset_version"):
        raise ValueError(f"routing URL and version provenance are required for {mode}")


def _validate_degradation(degradation, duration):
    if degradation.get("kind") not in VALID_DEGRADATIONS:
        raise ValueError("unknown GPS degradation")
    if degradation["kind"] == "impossible_jump":
        if not isinstance(degradation.get("at_s"), int) or not 0 <= degradation["at_s"] <= duration:
            raise ValueError("impossible jump must be inside its activity")
    elif not all(isinstance(degradation.get(name), int) for name in ("from_s", "until_s")) or not 0 <= degradation["from_s"] < degradation["until_s"] <= duration:
        raise ValueError("GPS interval degradation must be inside its activity")


def _activity_truth(activity, index, start_ms, duration):
    factual = {"index": index, "kind": activity["kind"], "from_ts_ms": start_ms, "until_ts_ms": start_ms + duration * 1000, "duration_s": duration}
    if activity["kind"] == "move":
        factual.update({"from": activity["from"], "to": activity["to"], "mode": activity["mode"]})
    else:
        factual.update({"at": activity["at"], "label": activity.get("label")})
    return factual


def _degradation_truth(degradations, activity_index, start_ms):
    output = []
    for degradation in degradations:
        entry = {"activity_index": activity_index, "kind": degradation["kind"]}
        if degradation["kind"] == "impossible_jump":
            entry["at_ts_ms"] = start_ms + degradation["at_s"] * 1000
        else:
            entry.update({"from_ts_ms": start_ms + degradation["from_s"] * 1000, "until_ts_ms": start_ms + degradation["until_s"] * 1000})
        output.append(entry)
    return output


def _write_json(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
