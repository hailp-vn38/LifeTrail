"""Create uploadable firmware-adaptive and dense baseline gps/1 fixture archives."""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import tempfile
import uuid
from zipfile import ZipFile, ZipInfo, ZIP_DEFLATED

from lifetrail_batch.batch_writer import encode_records, write_batch
from lifetrail_batch.firmware_policy import filter_observations, PersistReason
from lifetrail_batch.map_performance import observations
from lifetrail_batch.persistence_batches import partition_accepted

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SCENARIO = ROOT / "tools/scenarios/phase2-web-server-map-performance.json"
DEFAULT_OUTPUT = ROOT / "tools/fixtures/phase2-web-server-map-performance"


def write_archive(output, name, groups, seed):
    batches = []
    with tempfile.TemporaryDirectory() as directory:
        folder = Path(directory)
        for records in groups:
            body = encode_records(records)
            identity = hashlib.sha256(seed.encode() + name.encode() + body).digest()
            batch_id = str(uuid.UUID(bytes=identity[:16], version=4))
            batches.append(write_batch(folder, batch_id, records, body))
        with ZipFile(output / f"{name}.zip", "w") as archive:
            for path in sorted(folder.iterdir()):
                info = ZipInfo(path.name, date_time=(2026, 10, 5, 0, 0, 0))
                info.compress_type = ZIP_DEFLATED
                archive.writestr(info, path.read_bytes())
    return {"batch_count": len(batches), "record_count": sum(b.record_count for b in batches)}


def generate(scenario, output):
    baseline, truth = observations(scenario)
    accepted, metrics = filter_observations(baseline)
    if not accepted:
        raise ValueError("firmware policy persisted no records")
    output.mkdir(parents=True, exist_ok=True)
    firmware = write_archive(output, "firmware", partition_accepted(accepted), scenario["seed"])
    # The legacy comparator stores every acquisition epoch, with 60-second batches.
    dense = write_archive(output, "dense", [baseline[i:i + 60] for i in range(0, len(baseline), 60)], scenario["seed"])
    policy_sources = ROOT / "firmware/esp32/components/lifetrail_gps_policy"
    policy_hash = hashlib.sha256()
    for path in sorted(policy_sources.rglob("*")):
        if path.suffix in (".c", ".h"):
            policy_hash.update(str(path.relative_to(policy_sources)).encode() + path.read_bytes())
    report = {"scenario_id": scenario["id"], "timezone": scenario["timezone"],
              "acquisition_hz": 1, "firmware": firmware, "dense": dense,
              "firmware_policy_source_sha256": policy_hash.hexdigest(),
              "firmware_metrics": metrics,
              "persist_reasons": dict(Counter(PersistReason(e["reason"]).name for e in accepted)),
              "raw_record_reduction_percent": round(100 * (1 - len(accepted) / len(baseline)), 2),
              "activities": truth}
    (output / "summary.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scenario", type=Path, default=DEFAULT_SCENARIO)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    report = generate(json.loads(args.scenario.read_text()), args.output)
    print(json.dumps({k: v for k, v in report.items() if k != "activities"}, indent=2))


if __name__ == "__main__":
    main()
