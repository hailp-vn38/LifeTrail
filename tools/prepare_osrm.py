#!/usr/bin/env python3
"""Prepare three versioned MLD datasets using one pinned official OSRM image."""

import argparse
import hashlib
import json
import shutil
import subprocess
from pathlib import Path

IMAGE = "ghcr.io/project-osrm/osrm-backend:v6.0.0@sha256:729461bcc9ae9e6aafa92c0f93db9b060a32e85d5e72092c01ae4a4a9f1eb564"
PROFILES = {"car": "car.lua", "bike": "bicycle.lua", "foot": "foot.lua"}


def prepare(
    pbf, output, dataset_version, dataset_name="test-region", flat=False, source_url=None
):
    root = output if flat else output / dataset_version
    for name in PROFILES:
        directory = root / name
        if directory.exists():
            raise ValueError(
                f"dataset directory already exists; choose a new version: {directory}"
            )
    with pbf.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    engine = subprocess.check_output(
        ["docker", "run", "--rm", IMAGE, "osrm-routed", "--version"], text=True
    ).strip()
    for name, lua in PROFILES.items():
        directory = (root / name).resolve()
        if directory.exists():
            raise ValueError(
                f"dataset directory already exists; choose a new version: {directory}"
            )
        directory.mkdir(parents=True)
        profile_pbf = directory / f"{dataset_name}.osm.pbf"
        shutil.copyfile(pbf, profile_pbf)
        prefix = ["docker", "run", "--rm", "-v", f"{directory}:/data", IMAGE]
        for args in (
            [
                "osrm-extract",
                "--threads",
                "2",
                "-p",
                f"/opt/{lua}",
                f"/data/{dataset_name}.osm.pbf",
            ],
            ["osrm-partition", "--threads", "2", f"/data/{dataset_name}.osrm"],
            ["osrm-customize", "--threads", "2", f"/data/{dataset_name}.osrm"],
        ):
            subprocess.run(prefix + args, check=True)
        metadata = {
            "engine": "osrm",
            "engine_version": engine,
            "image": IMAGE,
            "profile": name,
            "profile_script": lua,
            "dataset_version": dataset_version,
            "osm_pbf_sha256": digest,
            "dataset_name": dataset_name,
            "source_url": source_url,
        }
        (directory / "metadata.json").write_text(
            json.dumps(metadata, indent=2, sort_keys=True) + "\n"
        )
        profile_pbf.unlink()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pbf", required=True, type=Path)
    parser.add_argument("--output", default=Path("data/osrm"), type=Path)
    parser.add_argument("--dataset-version", required=True)
    parser.add_argument("--dataset-name", default="test-region")
    parser.add_argument(
        "--flat", action="store_true", help="Write profiles directly under output"
    )
    parser.add_argument("--source-url")
    args = parser.parse_args()
    if (
        not args.dataset_version
        or Path(args.dataset_version).name != args.dataset_version
        or args.dataset_version in (".", "..")
    ):
        parser.error("dataset-version must be one directory name")
    if not args.dataset_name or any(
        not c.isascii() or not (c.isalnum() or c in "_-") for c in args.dataset_name
    ):
        parser.error("dataset-name must contain only letters, digits, underscores or hyphens")
    prepare(
        args.pbf, args.output, args.dataset_version,
        args.dataset_name, args.flat, args.source_url,
    )


if __name__ == "__main__":
    main()
