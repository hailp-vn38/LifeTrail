import http.server
import json
import threading
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from lifetrail_batch.routed import generate_scenario
from lifetrail_batch.realistic import generate_realistic_day
from lifetrail_batch.inspection import inspect_batch


class RouteHandler(http.server.BaseHTTPRequestHandler):
    requests = []

    def do_GET(self):
        self.requests.append(self.path)
        payload = {
            "code": "Ok",
            "routes": [
                {
                    "geometry": {
                        "type": "LineString",
                        "coordinates": [
                            [7.42, 43.73],
                            [7.422, 43.731],
                            [7.424, 43.732],
                        ],
                    },
                    "distance": 400,
                    "duration": 30,
                }
            ],
        }
        self.send_response(200)
        self.end_headers()
        self.wfile.write(json.dumps(payload).encode())

    def log_message(self, *_):
        pass


class RoutedScenarioTest(unittest.TestCase):
    def test_route_http_geometry_produces_replayable_seeded_batches_in_historical_time(
        self,
    ):
        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), RouteHandler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        scenario = {
            "timezone": "Europe/Monaco",
            "started_at": "2026-10-05T08:00:00+02:00",
            "duration_s": 4,
            "profile": "car",
            "waypoints": [[7.42, 43.73], [7.424, 43.732]],
            "interval_s": 1,
            "seed": "fixture-seed",
        }
        provenance = {
            "engine_version": "6.0.0",
            "dataset_version": "monaco-test",
            "profile": "car",
        }
        try:
            with TemporaryDirectory() as first, TemporaryDirectory() as second:
                batches = generate_scenario(
                    scenario,
                    f"http://127.0.0.1:{server.server_port}",
                    provenance,
                    Path(first),
                    batch_records=3,
                )
                repeated = generate_scenario(
                    scenario,
                    f"http://127.0.0.1:{server.server_port}",
                    provenance,
                    Path(second),
                    batch_records=3,
                )
                self.assertEqual(
                    [b.body_path.read_bytes() for b in batches],
                    [b.body_path.read_bytes() for b in repeated],
                )
                self.assertEqual(
                    [b.batch_id for b in batches], [b.batch_id for b in repeated]
                )
                self.assertEqual(
                    sum(
                        inspect_batch(b.body_path, b.manifest_path)["record_count"]
                        for b in batches
                    ),
                    5,
                )
                records = [
                    json.loads(line)
                    for b in batches
                    for line in b.body_path.read_text().splitlines()
                ]
                self.assertEqual((records[0]["lon"], records[0]["lat"]), (7.42, 43.73))
                self.assertEqual(
                    (records[-1]["lon"], records[-1]["lat"]), (7.424, 43.732)
                )
                self.assertEqual(records[0]["ts_ms"], 1791180000000)
                self.assertEqual(records[-1]["ts_ms"] - records[0]["ts_ms"], 4000)
                evidence = json.loads((Path(first) / "route-evidence.json").read_text())
                self.assertEqual(evidence["dataset_version"], "monaco-test")
                self.assertEqual(evidence["normalized_result"]["code"], "Ok")
                self.assertIn("geometries=geojson", RouteHandler.requests[0])
        finally:
            server.shutdown()
            server.server_close()
            thread.join()

    def test_realistic_day_preserves_raw_failure_kinds_and_frozen_routing_evidence(self):
        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), RouteHandler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        scenario = {
            "id": "operator-day", "timezone": "Europe/Monaco", "seed": "day-seed",
            "started_at": "2026-10-05T23:58:00+02:00", "interval_s": 1,
            "places": {"home": [7.42, 43.73], "coffee": [7.424, 43.732], "office": [7.42, 43.73]},
            "activities": [
                {"kind": "move", "from": "home", "to": "coffee", "mode": "foot", "duration_s": 8,
                 "degradations": [{"kind": "poor_quality", "from_s": 2, "until_s": 4},
                                  {"kind": "impossible_jump", "at_s": 5, "north_m": 1000},
                                  {"kind": "gnss_loss", "from_s": 6, "until_s": 7}]},
                {"kind": "stop", "at": "coffee", "duration_s": 4, "label": "long stop"},
                {"kind": "move", "from": "coffee", "to": "office", "mode": "car", "duration_s": 8},
            ],
        }
        metadata = {
            "foot": {"engine_version": "6.0.0", "dataset_version": "monaco-test"},
            "car": {"engine_version": "6.0.0", "dataset_version": "monaco-test"},
        }
        try:
            with TemporaryDirectory() as first, TemporaryDirectory() as second:
                first_batches = generate_realistic_day(
                    scenario, {"foot": f"http://127.0.0.1:{server.server_port}", "car": f"http://127.0.0.1:{server.server_port}"}, metadata, Path(first), batch_records=100
                )
                second_batches = generate_realistic_day(
                    scenario, {"foot": f"http://127.0.0.1:{server.server_port}", "car": f"http://127.0.0.1:{server.server_port}"}, metadata, Path(second), batch_records=100
                )
                self.assertEqual([batch.body_path.read_bytes() for batch in first_batches], [batch.body_path.read_bytes() for batch in second_batches])
                records = [json.loads(line) for batch in first_batches for line in batch.body_path.read_text().splitlines()]
                self.assertTrue(any(record["hdop"] >= 10 for record in records))
                self.assertFalse(any(record["ts_ms"] == records[0]["ts_ms"] + 6000 for record in records))
                truth = json.loads((Path(first) / "ground-truth.json").read_text())
                self.assertEqual([item["kind"] for item in truth["degradations"]], ["poor_quality", "impossible_jump", "gnss_loss"])
                self.assertEqual(truth["activities"][0]["mode"], "foot")
                evidence = json.loads((Path(first) / "routing-evidence.json").read_text())
                self.assertEqual([item["profile"] for item in evidence["routes"]], ["foot", "car"])
                self.assertEqual(evidence["routes"][0]["normalized_result"]["code"], "Ok")
        finally:
            server.shutdown()
            server.server_close()
            thread.join()
