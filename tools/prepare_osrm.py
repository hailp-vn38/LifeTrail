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


def prepare(pbf, output, dataset_version):
    for name in PROFILES:
        directory = output / dataset_version / name
        if directory.exists():
            raise ValueError(f"dataset directory already exists; choose a new version: {directory}")
    digest = hashlib.sha256(pbf.read_bytes()).hexdigest()
    engine = subprocess.check_output(
        ["docker", "run", "--rm", IMAGE, "osrm-routed", "--version"], text=True
    ).strip()
    for name, lua in PROFILES.items():
        directory = (output / dataset_version / name).resolve()
        if directory.exists():
            raise ValueError(
                f"dataset directory already exists; choose a new version: {directory}"
            )
        directory.mkdir(parents=True)
        shutil.copyfile(pbf, directory / "test-region.osm.pbf")
        prefix = ["docker", "run", "--rm", "-v", f"{directory}:/data", IMAGE]
        for args in (
            [
                "osrm-extract",
                "--threads",
                "2",
                "-p",
                f"/opt/{lua}",
                "/data/test-region.osm.pbf",
            ],
            ["osrm-partition", "--threads", "2", "/data/test-region.osrm"],
            ["osrm-customize", "--threads", "2", "/data/test-region.osrm"],
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
        }
        (directory / "metadata.json").write_text(
            json.dumps(metadata, indent=2, sort_keys=True) + "\n"
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pbf", required=True, type=Path)
    parser.add_argument("--output", default=Path("data/osrm"), type=Path)
    parser.add_argument("--dataset-version", required=True)
    args = parser.parse_args()
    if (
        not args.dataset_version
        or Path(args.dataset_version).name != args.dataset_version
        or args.dataset_version in (".", "..")
    ):
        parser.error("dataset-version must be one directory name")
    prepare(args.pbf, args.output, args.dataset_version)


if __name__ == "__main__":
    main()
