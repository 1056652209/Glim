#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
export GLIM_SPARKLE_DIR="${GLIM_SPARKLE_DIR:-$PWD/target/sparkle/2.10.0}"
[[ -d "$GLIM_SPARKLE_DIR/Sparkle.framework" ]] || scripts/fetch-sparkle.sh
updater_test_dir=$(mktemp -d)
trap 'rm -rf "$updater_test_dir"' EXIT
xcrun swiftc -swift-version 5 -F "$GLIM_SPARKLE_DIR" -framework Sparkle \
    -Xlinker -rpath -Xlinker "$GLIM_SPARKLE_DIR" \
    native/Updater.swift native/tests/UpdaterTests.swift -o "$updater_test_dir/updater-tests"
"$updater_test_dir/updater-tests"

python3 scripts/test-updater-configuration.py
python3 scripts/test-macos-signing.py

python3 scripts/test-windows-feeds.py
