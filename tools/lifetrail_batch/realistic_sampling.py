"""Deterministic physical GPS observations for operator scenarios."""

from __future__ import annotations

import bisect
import math
import random

from .route_sampling import distance, validate_coordinates


def moving_records(coordinates, start_ms, duration_s, interval_s, seed):
    """Sample a route on a variable-speed clock independent from OSRM duration."""
    validate_coordinates(coordinates)
    distances = [distance(a, b) for a, b in zip(coordinates, coordinates[1:])]
    progress = [0.0]
    for length in distances:
        progress.append(progress[-1] + length)
    if progress[-1] <= 0:
        raise ValueError("routed movement must have positive distance")

    offsets = list(range(0, duration_s, interval_s)) + [duration_s]
    weights = _speed_weights(duration_s, interval_s, seed)
    cumulative = [0.0]
    for weight in weights:
        cumulative.append(cumulative[-1] + weight)
    total_weight = cumulative[-1]
    records = []
    for index, seconds in enumerate(offsets):
        target = progress[-1] * cumulative[index] / total_weight
        point_index = min(bisect.bisect_right(progress, target) - 1, len(coordinates) - 2)
        ratio = (target - progress[point_index]) / distances[point_index]
        lon, lat = _interpolate(coordinates[point_index], coordinates[point_index + 1], ratio)
        next_seconds = offsets[min(index + 1, len(offsets) - 1)]
        next_target = progress[-1] * cumulative[min(index + 1, len(cumulative) - 1)] / total_weight
        speed = 0.0 if next_seconds == seconds else (next_target - target) / (next_seconds - seconds)
        records.append(_record(start_ms + seconds * 1000, lon, lat, speed, seed, index))
    return records


def stopped_records(coordinate, start_ms, duration_s, interval_s, seed):
    """Generate small seeded drift without asserting a derived Stop."""
    validate_coordinates([coordinate, coordinate])
    offsets = list(range(0, duration_s, interval_s)) + [duration_s]
    randomizer = random.Random(f"{seed}:stationary")
    records = []
    for index, seconds in enumerate(offsets):
        phase = index / max(1, len(offsets) - 1)
        east_m = math.sin(phase * math.pi * 4 + randomizer.random()) * 1.2
        north_m = math.cos(phase * math.pi * 3 + randomizer.random()) * 1.2
        lon, lat = coordinate
        lat += north_m / 111_320
        lon += east_m / (111_320 * math.cos(math.radians(lat)))
        records.append(_record(start_ms + seconds * 1000, lon, lat, 0.03, seed, index))
    return records


def degrade(records, degradations, interval_s):
    """Apply protocol-valid failures and return only observations that exist."""
    output = []
    for index, record in enumerate(records):
        offset = index * interval_s
        altered = dict(record)
        omit = False
        for degradation in degradations:
            kind = degradation["kind"]
            if kind == "gnss_loss" and degradation["from_s"] <= offset < degradation["until_s"]:
                omit = True
            elif kind == "poor_quality" and degradation["from_s"] <= offset < degradation["until_s"]:
                altered.update({"fix_quality": 1, "satellites": 3, "hdop": 12.5})
            elif kind == "impossible_jump" and offset == degradation["at_s"]:
                altered["lat"] = round(altered["lat"] + degradation.get("north_m", 1000) / 111_320, 7)
                altered["lon"] = round(altered["lon"] + degradation.get("east_m", 0) / 111_320, 7)
        if not omit:
            output.append(altered)
    return output


def _speed_weights(duration_s, interval_s, seed):
    count = len(range(0, duration_s, interval_s))
    randomizer = random.Random(f"{seed}:speed")
    weights = []
    for index in range(count):
        phase = (index + 1) / max(1, count)
        acceleration = 0.45 + 0.55 * math.sin(math.pi * phase)
        variation = 0.15 * math.sin(index * 0.73 + randomizer.random())
        weights.append(max(0.08, acceleration + variation))
    return weights


def _interpolate(a, b, ratio):
    return a[0] + (b[0] - a[0]) * ratio, a[1] + (b[1] - a[1]) * ratio


def _record(timestamp, lon, lat, speed, seed, index):
    randomizer = random.Random(f"{seed}:quality:{index}")
    return {
        "ts_ms": timestamp,
        "lat": round(lat, 7),
        "lon": round(lon, 7),
        "speed_mps": round(speed, 3),
        "fix_quality": 1,
        "satellites": randomizer.randint(8, 13),
        "hdop": round(randomizer.uniform(0.7, 1.3), 2),
    }
