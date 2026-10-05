import argparse
import json
import sys
from pathlib import Path

from .inspection import inspect_batch
from .replay import commit_and_replay_batch, replay_batch


def main() -> int:
    parser = argparse.ArgumentParser(description="Inspect or replay immutable LifeTrail Batches.")
    subcommands = parser.add_subparsers(dest="command", required=True)
    for name in ("inspect", "replay", "commit-and-replay"):
        command = subcommands.add_parser(name)
        command.add_argument("--body", type=Path, required=True)
        command.add_argument("--manifest", type=Path, required=True)
    for name in ("replay", "commit-and-replay"):
        command = subcommands.choices[name]
        command.add_argument("--endpoint", required=True)
        command.add_argument("--token", required=True)
    args = parser.parse_args()
    try:
        if args.command == "inspect":
            result = inspect_batch(args.body, args.manifest)
        elif args.command == "replay":
            result = replay_batch(args.endpoint, args.token, args.body, args.manifest)
        else:
            result = commit_and_replay_batch(args.endpoint, args.token, args.body, args.manifest)
    except (OSError, ValueError) as error:
        print(f"batch acceptance failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0
