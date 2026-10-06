import http.server
import json
import threading
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from lifetrail_batch.routed import generate_scenario
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
