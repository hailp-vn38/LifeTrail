"""HTTP boundary for simulator routing; the browser never uses this client."""

import json
import urllib.parse
import urllib.request

SERVICE_PROFILE = {"car": "driving", "bike": "cycling", "foot": "walking"}


def route(base_url, profile, waypoints):
    coordinates = ";".join(f"{lon:.7f},{lat:.7f}" for lon, lat in waypoints)
    parameters = urllib.parse.urlencode(
        {"overview": "full", "geometries": "geojson", "steps": "false"}
    )
    url = f"{base_url.rstrip('/')}/route/v1/{SERVICE_PROFILE[profile]}/{coordinates}?{parameters}"
    with urllib.request.urlopen(url, timeout=30) as response:
        result = json.load(response)
    if result.get("code") != "Ok" or not result.get("routes"):
        raise ValueError(f"OSRM Route failed: {result.get('code', 'invalid result')}")
    geometry = result["routes"][0].get("geometry", {})
    if geometry.get("type") != "LineString":
        raise ValueError("OSRM Route did not return a LineString")
    return geometry.get("coordinates"), {
        "request_url": url,
        "normalized_result": result,
    }
