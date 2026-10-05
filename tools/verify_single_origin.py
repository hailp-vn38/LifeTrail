#!/usr/bin/env python3
"""Verify the built Web app and canonical Daily View share one server origin."""

import argparse
import re
import sys
import urllib.error
import urllib.parse
import urllib.request


def fetch(url: str) -> tuple[str, bytes]:
    try:
        with urllib.request.urlopen(url, timeout=15) as response:
            if response.status != 200:
                raise ValueError(f"{url} returned HTTP {response.status}")
            return response.headers.get_content_type(), response.read()
    except urllib.error.HTTPError as error:
        raise ValueError(f"{url} returned HTTP {error.code}") from error


def verify(origin: str, device_id: str, date: str) -> None:
    origin = origin.rstrip("/")
    root_type, root = fetch(f"{origin}/")
    daily_type, daily = fetch(f"{origin}/devices/{device_id}/day/{date}")
    if root_type != "text/html" or daily_type != "text/html":
        raise ValueError("root and canonical Daily View must be HTML")
    if root != daily or b'<div id="app"></div>' not in root:
        raise ValueError("canonical Daily View is not the built SPA entrypoint")
    match = re.search(rb'<script[^>]+src="([^"]+)"', root)
    if match is None:
        raise ValueError("built SPA entrypoint has no script asset")
    asset_url = urllib.parse.urljoin(f"{origin}/", match.group(1).decode("ascii"))
    _, asset = fetch(asset_url)
    if not asset:
        raise ValueError("built SPA script asset is empty")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--origin", required=True)
    parser.add_argument("--device-id", required=True)
    parser.add_argument("--date", required=True)
    args = parser.parse_args()
    try:
        verify(args.origin, args.device_id, args.date)
    except ValueError as error:
        print(f"single-origin verification failed: {error}", file=sys.stderr)
        return 1
    print("single-origin verification passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
