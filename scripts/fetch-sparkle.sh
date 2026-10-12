#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
# Pin the official distribution (including its updater helpers and signing tools).
sparkle_version=2.10.0
sparkle_sha256=c2bf58aa8387266ac179357b1415d6f2635f044da8be41042af32425dae6da0c
sparkle_dir="${GLIM_SPARKLE_DIR:-$PWD/target/sparkle/$sparkle_version}"
mkdir -p "$sparkle_dir"
archive="$sparkle_dir/Sparkle.tar.xz"
if [[ ! -f "$archive" ]]; then
    curl --fail --location --retry 3 --connect-timeout 20 --max-time 300 \
        "https://github.com/sparkle-project/Sparkle/releases/download/$sparkle_version/Sparkle-$sparkle_version.tar.xz" \
        --output "$archive.partial"
    mv "$archive.partial" "$archive"
fi
actual=$(shasum -a 256 "$archive" | cut -d ' ' -f 1)
[[ "$actual" == "$sparkle_sha256" ]] || { echo 'Sparkle archive checksum mismatch.' >&2; exit 1; }
# Re-extract from verified bytes rather than trusting a mutable cached framework.
tar -xJf "$archive" -C "$sparkle_dir"
printf '%s\n' "$sparkle_dir"
