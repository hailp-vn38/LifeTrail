import json
import tempfile
import unittest
from pathlib import Path

from lifetrail_batch.simulate import generate_route
from lifetrail_batch.firmware_policy import filter_observations
from lifetrail_batch.persistence_scenarios import scenario


class FirmwarePersistenceTest(unittest.TestCase):
    def test_simulator_uses_real_policy_and_preserves_source_records(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            baseline = generate_route(root / "baseline", date="2026-10-05")
            filtered = generate_route(root / "filtered", date="2026-10-05", firmware_persistence=True)
            source = {record["ts_ms"]: record for batch in baseline
                      for record in map(json.loads, batch.body_path.read_text().splitlines())}
            records = [record for batch in filtered
                       for record in map(json.loads, batch.body_path.read_text().splitlines())]
            self.assertLess(len(records), len(source) / 2)
            for record in records:
                self.assertEqual(record, source[record["ts_ms"]])
            self.assertTrue(all(batch.last_ts_ms - batch.first_ts_ms < 60000 for batch in filtered))
            self.assertTrue(set(batch.batch_id for batch in filtered).isdisjoint(
                batch.batch_id for batch in baseline))

    def test_no_fix_produces_no_record_inside_the_gap(self):
        accepted, metrics = filter_observations(scenario("no_fix"))
        start = 1791158400000
        self.assertTrue(all(event["record"]["ts_ms"] < start + 600000 or
                            event["record"]["ts_ms"] >= start + 29400000 for event in accepted))
        self.assertEqual(metrics["no_fix_periods_total_s"], 28800)


if __name__ == "__main__":
    unittest.main()
