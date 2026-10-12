#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
preview_test_dir=$(mktemp -d)
trap 'rm -rf "$preview_test_dir"' EXIT
xcrun swiftc -swift-version 5 \
    native/Preview.swift native/SQLitePreview.swift native/EmbeddedPreview.swift \
    native/tests/EmbeddedPreviewTests.swift -o "$preview_test_dir/preview-tests"
"$preview_test_dir/preview-tests"
