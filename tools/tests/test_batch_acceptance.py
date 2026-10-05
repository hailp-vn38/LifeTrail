import hashlib
import http.server
import json
import threading
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from lifetrail_batch import commit_and_replay_batch, inspect_batch, replay_batch


FIXTURE_DIRECTORY = Path(__file__).parents[1] / "fixtures" / "lan-acceptance"
FIXTURE_BODY = next(FIXTURE_DIRECTORY.glob("*.ndjson.ready"))
FIXTURE_MANIFEST = FIXTURE_BODY.with_suffix("").with_suffix(".manifest")
BODY = FIXTURE_BODY.read_bytes()
BATCH_ID = FIXTURE_BODY.name.removesuffix(".ndjson.ready")


class RecordingHandler(http.server.BaseHTTPRequestHandler):
    requests = []

    def do_POST(self):
        body = self.rfile.read(int(self.headers["Content-Length"]))
        self.__class__.requests.append((self.path, dict(self.headers), body))
        duplicate = len(self.__class__.requests) >= 2
        response = json.dumps(
            {
                "batch_id": BATCH_ID,
                "status": "committed",
                "record_count": 2,
                "duplicate": duplicate,
            }
        ).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(response)))
        self.end_headers()
        self.wfile.write(response)

    def log_message(self, format, *args):
        pass


class BatchAcceptanceTest(unittest.TestCase):
    def setUp(self):
        self.temp = TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.body_path = self.root / f"{BATCH_ID}.ndjson.ready"
        self.manifest_path = self.root / f"{BATCH_ID}.manifest"
        self.body_path.write_bytes(FIXTURE_BODY.read_bytes())
        self.manifest_path.write_bytes(FIXTURE_MANIFEST.read_bytes())

    def tearDown(self):
        self.temp.cleanup()

    def test_inspect_accepts_exact_manifest_and_strict_ndjson(self):
        summary = inspect_batch(self.body_path, self.manifest_path)

        self.assertEqual(summary["batch_id"], BATCH_ID)
        self.assertEqual(summary["record_count"], 2)
        self.assertEqual(summary["sha256"], hashlib.sha256(BODY).hexdigest())

    def test_inspect_rejects_manifest_or_framing_mismatch(self):
        self.manifest_path.write_text(self.manifest_path.read_text().replace("record_count=2", "record_count=3"), encoding="ascii")
        with self.assertRaisesRegex(ValueError, "record_count"):
            inspect_batch(self.body_path, self.manifest_path)

        self.body_path.write_bytes(BODY.replace(b"\n", b"\r\n"))
        with self.assertRaisesRegex(ValueError, "CRLF"):
            inspect_batch(self.body_path, self.manifest_path)

    def test_commit_then_replay_posts_the_original_bytes_twice(self):
        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), RecordingHandler)
        RecordingHandler.requests = []
        thread = threading.Thread(target=server.serve_forever)
        thread.start()
        try:
            endpoint = f"http://127.0.0.1:{server.server_port}/api/v1/device/batches"
            responses = commit_and_replay_batch(
                endpoint, "lt_dev_test", self.body_path, self.manifest_path
            )

            self.assertEqual([response["duplicate"] for response in responses], [False, True])
            self.assertEqual(len(RecordingHandler.requests), 2)
            for path, headers, body in RecordingHandler.requests:
                self.assertEqual(path, "/api/v1/device/batches")
                self.assertEqual(body, BODY)
                self.assertEqual(headers["X-Lifetrail-Batch-Id"], BATCH_ID)
                self.assertEqual(headers["X-Lifetrail-Content-Sha256"], hashlib.sha256(BODY).hexdigest())
        finally:
            server.shutdown()
            thread.join()
            server.server_close()

    def test_replay_requires_a_previously_committed_batch_to_stay_duplicate(self):
        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), RecordingHandler)
        RecordingHandler.requests = [("already", {}, BODY)]
        thread = threading.Thread(target=server.serve_forever)
        thread.start()
        try:
            endpoint = f"http://127.0.0.1:{server.server_port}/api/v1/device/batches"
            responses = replay_batch(endpoint, "lt_dev_test", self.body_path, self.manifest_path)

            self.assertEqual([response["duplicate"] for response in responses], [True, True])
        finally:
            server.shutdown()
            thread.join()
            server.server_close()


if __name__ == "__main__":
    unittest.main()
