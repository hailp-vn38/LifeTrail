"""Sample a routed LineString in explicitly chosen historical time."""

import bisect
import math
import random


def validate_coordinates(coordinates):
    if not isinstance(coordinates, list) or len(coordinates) < 2:
        raise ValueError("route needs at least two coordinates")
    for coordinate in coordinates:
        if not isinstance(coordinate, (list, tuple)) or len(coordinate) != 2:
            raise ValueError("coordinates must be longitude/latitude pairs")
        lon, lat = coordinate
        if not all(
            isinstance(v, (int, float)) and not isinstance(v, bool) and math.isfinite(v)
            for v in coordinate
        ):
            raise ValueError("coordinates must be finite numbers")
        if not -180 <= lon <= 180 or not -90 <= lat <= 90:
            raise ValueError("coordinates outside GPS range")


def distance(a, b):
    lon1, lat1, lon2, lat2 = map(math.radians, (*a, *b))
    chord = (
        math.sin((lat2 - lat1) / 2) ** 2
        + math.cos(lat1) * math.cos(lat2) * math.sin((lon2 - lon1) / 2) ** 2
    )
    return 6371000 * 2 * math.asin(math.sqrt(min(1, chord)))


def sample(coordinates, start_ms, duration_s, interval_s, seed):
    validate_coordinates(coordinates)
    progress = [0.0]
    for a, b in zip(coordinates, coordinates[1:]):
        progress.append(progress[-1] + distance(a, b))
    if progress[-1] <= 0:
        raise ValueError("routed movement must have positive distance")
    rng = random.Random(seed)
    offsets = list(range(0, duration_s, interval_s)) + [duration_s]
    records = []
    for seconds in offsets:
        target = progress[-1] * seconds / duration_s
        index = min(bisect.bisect_right(progress, target) - 1, len(coordinates) - 2)
        a, b = coordinates[index : index + 2]
        length = progress[index + 1] - progress[index]
        ratio = (target - progress[index]) / length if length else 0
        lon, lat = (a[i] + (b[i] - a[i]) * ratio for i in (0, 1))
        records.append(
            {
                "ts_ms": start_ms + seconds * 1000,
                "lat": round(lat, 7),
                "lon": round(lon, 7),
                "speed_mps": round(progress[-1] / duration_s, 3),
                "fix_quality": 1,
                "satellites": rng.randint(8, 12),
                "hdop": round(rng.uniform(0.7, 1.2), 2),
            }
        )
    return records
