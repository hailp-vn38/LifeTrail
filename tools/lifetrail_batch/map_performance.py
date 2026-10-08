"""Offline observations for comparing dense acquisition with real firmware storage."""

from datetime import datetime
from zoneinfo import ZoneInfo

from .realistic_sampling import degrade, moving_records, stopped_records
from .route_sampling import validate_coordinates
from .simulate import _bearing_deg


def observations(scenario):
    start = datetime.fromisoformat(scenario["started_at"])
    if start.tzinfo is None or start.utcoffset() != start.astimezone(ZoneInfo(scenario["timezone"])).utcoffset():
        raise ValueError("started_at must have an offset matching timezone")
    current_ms = int(start.timestamp() * 1000)
    records, truth = [], []
    for index, activity in enumerate(scenario["activities"]):
        duration = activity["duration_s"]
        if type(duration) is not int or duration <= 0:
            raise ValueError("activity duration must be a positive integer")
        seed = f"{scenario['seed']}:{index}"
        if activity["kind"] == "move":
            generated = moving_records(activity["coordinates"], current_ms, duration, 1, seed)
            for record, following in zip(generated, generated[1:]):
                record["course_deg"] = _bearing_deg(
                    (record["lat"], record["lon"]),
                    (following["lat"], following["lon"]),
                )
        elif activity["kind"] == "stop":
            validate_coordinates([activity["at"], activity["at"]])
            generated = stopped_records(activity["at"], current_ms, duration, 1, seed)
        else:
            raise ValueError("activity kind must be move or stop")
        failures = activity.get("degradations", [])
        for failure in failures:
            if (failure["kind"] != "gnss_loss" or
                    not 0 <= failure["from_s"] < failure["until_s"] <= duration):
                raise ValueError("GNSS loss must be within its activity")
        # The next activity owns its boundary observation, avoiding duplicate epochs.
        records.extend(degrade(generated[:-1], failures, 1))
        truth.append({**activity, "from_ts_ms": current_ms,
                      "until_ts_ms": current_ms + duration * 1000})
        current_ms += duration * 1000
    if not records:
        raise ValueError("scenario needs observable GPS records")
    return records, truth
