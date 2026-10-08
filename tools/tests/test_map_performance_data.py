import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from zipfile import ZipFile

from generate_map_performance_data import DEFAULT_SCENARIO, generate
from lifetrail_batch.firmware_policy import filter_observations, PersistReason
from lifetrail_batch.inspection import inspect_batch
from lifetrail_batch.map_performance import observations


class MapPerformanceDataTest(unittest.TestCase):
    def test_real_firmware_archives_preserve_observations_and_gap_boundaries(self):
        scenario = json.loads(DEFAULT_SCENARIO.read_text())
        source, activities = observations(scenario)
        by_time = {r["ts_ms"]: r for r in source}
        self.assertEqual(len(by_time), len(source))
        accepted, _ = filter_observations(source)
        expected = [e["record"] for e in accepted]
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            report = generate(scenario, output)
            self.assertLess(report["firmware"]["record_count"], len(source) / 10)
            for name, records in [("firmware", expected), ("dense", source)]:
                folder = output / name
                with ZipFile(output / f"{name}.zip") as archive:
                    archive.extractall(folder)
                restored = []
                for manifest in folder.glob("*.manifest"):
                    body = manifest.with_suffix(".ndjson.ready")
                    metadata = inspect_batch(body, manifest)
                    self.assertLess(metadata["last_ts_ms"] - metadata["first_ts_ms"],
                                    300000 if name == "firmware" else 60000)
                    restored.extend(map(json.loads, body.read_text().splitlines()))
                self.assertEqual(sorted(restored, key=lambda r: r["ts_ms"]), records)
            before = {name: hashlib.sha256((output / f"{name}.zip").read_bytes()).hexdigest()
                      for name in ("firmware", "dense")}
            generate(scenario, output)
            self.assertEqual(before, {name: hashlib.sha256((output / f"{name}.zip").read_bytes()).hexdigest()
                                      for name in before})
        times = {r["ts_ms"] for r in expected}
        for activity in activities:
            for loss in activity.get("degradations", []):
                start = activity["from_ts_ms"] + loss["from_s"] * 1000
                end = activity["from_ts_ms"] + loss["until_s"] * 1000
                self.assertFalse(any(start <= t < end for t in times))
                self.assertIn(start - 1000, times)
                self.assertIn(end, times)
        stationary = [e["record"]["ts_ms"] for e in accepted
                      if e["reason"] == PersistReason.HEARTBEAT
                      and e["record"]["ts_ms"] < activities[0]["until_ts_ms"]]
        self.assertGreater(len(stationary), 100)
        self.assertTrue(all(b - a == 120000 for a, b in zip(stationary, stationary[1:])))
        self.assertGreater(report["persist_reasons"]["HEADING"], 0)
        self.assertGreater(report["persist_reasons"]["BACKFILL"], 0)
        self.assertGreater(report["persist_reasons"]["FIX_RECOVERY"], 0)


if __name__ == "__main__":
    unittest.main()
