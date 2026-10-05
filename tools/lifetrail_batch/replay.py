import json
import urllib.error
import urllib.request
from pathlib import Path

from .inspection import inspect_batch


def commit_and_replay_batch(endpoint: str, token: str, body_path: Path, manifest_path: Path) -> list[dict[str, object]]:
    return post_twice(endpoint, token, body_path, manifest_path, [False, True])


def replay_batch(endpoint: str, token: str, body_path: Path, manifest_path: Path) -> list[dict[str, object]]:
    return post_twice(endpoint, token, body_path, manifest_path, [True, True])


def post_twice(endpoint: str, token: str, body_path: Path, manifest_path: Path, expected_duplicates: list[bool]) -> list[dict[str, object]]:
    manifest = inspect_batch(body_path, manifest_path)
    body = body_path.read_bytes()
    headers = {
        "Authorization": f"Bearer {token}", "Content-Type": "application/x-ndjson",
        "X-LifeTrail-Batch-Id": str(manifest["batch_id"]), "X-LifeTrail-Schema": "gps/1",
        "X-LifeTrail-Content-SHA256": str(manifest["sha256"]),
        "X-LifeTrail-Byte-Length": str(manifest["byte_length"]),
        "X-LifeTrail-Record-Count": str(manifest["record_count"]),
    }
    responses = [post(endpoint, headers, body), post(endpoint, headers, body)]
    for response, duplicate in zip(responses, expected_duplicates, strict=True):
        if (response.get("batch_id") != manifest["batch_id"] or response.get("status") != "committed"
                or response.get("record_count") != manifest["record_count"] or response.get("duplicate") is not duplicate):
            raise ValueError(f"unexpected commit response: {response!r}")
    return responses


def post(endpoint: str, headers: dict[str, str], body: bytes) -> dict[str, object]:
    request = urllib.request.Request(endpoint, data=body, headers=headers, method="POST")
    try:
        with urllib.request.urlopen(request, timeout=15) as response:
            if response.status != 200:
                raise ValueError(f"server returned HTTP {response.status}")
            return json.loads(response.read())
    except urllib.error.HTTPError as error:
        raise ValueError(f"server returned HTTP {error.code}") from error
