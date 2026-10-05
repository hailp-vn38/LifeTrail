import json
import tempfile
import unittest
import uuid
from pathlib import Path

from lifetrail_batch.inspection import inspect_batch
from lifetrail_batch.simulate import generate_route


class SimulatedRouteTest(unittest.TestCase):
    def test_generates_three_deterministic_valid_batches(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first = generate_route(root, date="2026-10-05")
            first_ids = [batch.batch_id for batch in first]

            self.assertEqual(len(first), 3)
            self.assertEqual(sum(batch.record_count for batch in first), 900)

            for batch in first:
                uuid_value = uuid.UUID(batch.batch_id)
                self.assertEqual(uuid_value.version, 4)
                manifest = inspect_batch(batch.body_path, batch.manifest_path)
                self.assertEqual(manifest["record_count"], 300)
                self.assertLess(manifest["byte_length"], 262_144)

            second = generate_route(root, date="2026-10-05")
            self.assertEqual([batch.batch_id for batch in second], first_ids)

    def test_route_contains_valid_gps_shape_and_increasing_timestamps(self):
        with tempfile.TemporaryDirectory() as directory:
            batches = generate_route(Path(directory), date="2026-10-05", seconds=12, batch_records=5)
            records = []
            for batch in batches:
                for line in batch.body_path.read_text(encoding="ascii").splitlines():
                    records.append(json.loads(line))

            timestamps = [record["ts_ms"] for record in records]
            self.assertEqual(timestamps, sorted(timestamps))
            self.assertEqual(len(set(timestamps)), len(timestamps))
            self.assertTrue(all(record["fix_quality"] > 0 for record in records))
            self.assertTrue(all(-90 <= record["lat"] <= 90 for record in records))
            self.assertTrue(all(-180 <= record["lon"] <= 180 for record in records))


if __name__ == "__main__":
    unittest.main()
