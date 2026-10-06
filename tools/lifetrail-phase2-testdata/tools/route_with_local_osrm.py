#!/usr/bin/env python3
"""
Optional helper for replacing fallback test polylines with routes from a locally
running OSRM instance. It intentionally uses only the Python standard library.

Example:
  python tools/route_with_local_osrm.py \
    --base-url http://localhost:5000 \
    --profile driving \
    --coordinates "106.6895,10.7821;106.7001,10.7758"
"""
import argparse, json, urllib.parse, urllib.request

p = argparse.ArgumentParser()
p.add_argument("--base-url", required=True)
p.add_argument("--profile", default="driving")
p.add_argument("--coordinates", required=True)
args = p.parse_args()
coords = urllib.parse.quote(args.coordinates, safe=",;")
url = f"{args.base_url.rstrip('/')}/route/v1/{args.profile}/{coords}?overview=full&geometries=geojson"
with urllib.request.urlopen(url, timeout=20) as r:
    payload = json.load(r)
if payload.get("code") != "Ok":
    raise SystemExit(json.dumps(payload))
print(json.dumps(payload["routes"][0]["geometry"], separators=(",", ":")))
