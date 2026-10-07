"""Deterministic 1 Hz observations with known movement and no-fix periods."""

import math

START_MS = 1791158400000  # 2026-10-05T00:00:00Z
METERS_PER_DEGREE = 111194.9266


def scenario(name: str) -> list[dict]:
    durations = {"stationary": 28800, "lost_heartbeat": 3600, "walk": 900, "drive": 900,
                 "turn": 900, "commute": 1800, "short_stop": 900,
                 "no_fix": 30000}
    x = y = 0.0
    records = []
    for second in range(durations[name]):
        speed = 0.0
        course = 0.0
        if name in ("walk", "drive", "turn", "short_stop"):
            speed = 1.4 if name == "walk" else 15.0
            if name == "short_stop" and 300 <= second < 320:
                speed = 0.0
            if name == "turn" and second >= 300:
                course = 90.0
        elif name == "commute" and (600 <= second < 800 or second >= 1400):
            speed = 5.0
        if name == "no_fix" and 600 <= second < 29400:
            continue
        x += speed * math.sin(math.radians(course))
        y += speed * math.cos(math.radians(course))
        jitter = math.sin(second / 17.0) * 1.0 if speed == 0.0 else 0.0
        records.append({
            "ts_ms": START_MS + second * 1000,
            "lat": 10.0 + (y + jitter) / METERS_PER_DEGREE,
            "lon": 106.0 + x / (METERS_PER_DEGREE * math.cos(math.radians(10.0))),
            "speed_mps": speed, "course_deg": course, "alt_m": 8.0,
            "hdop": 1.0, "fix_quality": 1, "satellites": 10,
        })
    return records
