#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
source_dir="${1:-$repo_root/data/osrm/source}"
source_url="https://download.geofabrik.de/asia/vietnam-latest.osm.pbf"
mkdir -p "$source_dir"
cd "$source_dir"

# Keep the completed source intact until the new download passes verification.
temp_dir="$(mktemp -d .vietnam-download.XXXXXX)"
trap 'rm -rf -- "$temp_dir"' EXIT
curl --fail --location --silent --show-error --retry 3 \
  --dump-header "$temp_dir/headers.txt" \
  --output "$temp_dir/vietnam-latest.osm.pbf" "$source_url"
curl --fail --location --silent --show-error --retry 3 \
  --output "$temp_dir/vietnam-latest.osm.pbf.md5" "$source_url.md5"
(cd "$temp_dir" && md5sum --check vietnam-latest.osm.pbf.md5)
mv "$temp_dir/vietnam-latest.osm.pbf" .
mv "$temp_dir/vietnam-latest.osm.pbf.md5" .
mv "$temp_dir/headers.txt" vietnam-download-headers.txt
sha256sum vietnam-latest.osm.pbf
