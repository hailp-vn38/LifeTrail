"""Generate baseline/Device-filtered evidence for Phase 2 server acceptance."""

import argparse
import json
from pathlib import Path

from lifetrail_batch.firmware_policy import PersistReason, filter_observations, policy_binary
from lifetrail_batch.persistence_scenarios import scenario


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    binary = policy_binary()
    summary = []
    for name in ("stationary", "lost_heartbeat", "walk", "drive", "turn", "commute", "short_stop", "no_fix"):
        baseline = scenario(name)
        accepted, metrics = filter_observations(baseline, binary)
        if name == "lost_heartbeat":
            heartbeat_indices = [i for i, event in enumerate(accepted)
                                 if event["reason"] == PersistReason.HEARTBEAT]
            del accepted[heartbeat_indices[5]]
        filtered = [event["record"] for event in accepted]
        result = {"name": name, "baseline": baseline, "filtered": filtered,
                  "metrics": metrics}
        (args.output / f"{name}.json").write_text(json.dumps(result), encoding="utf-8")
        summary.append({"name": name, **metrics,
                        "persist_ratio": len(filtered) / len(baseline)})
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
