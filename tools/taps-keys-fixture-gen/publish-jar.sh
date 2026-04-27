#!/usr/bin/env bash
set -euo pipefail

REPO="Skyscanner/taps-keys-fixtures"
TAG="v1"
JAR_NAME="taps-keys-fixture-gen.jar"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
JAR_PATH="$SCRIPT_DIR/build/libs/$JAR_NAME"

echo "=== Building shadow JAR ==="
cd "$SCRIPT_DIR"
./gradlew shadowJar

if [ ! -f "$JAR_PATH" ]; then
    echo "FATAL: JAR not found at $JAR_PATH"
    exit 1
fi

echo "=== Checking for existing release $TAG on $REPO ==="
if gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1; then
    # Release exists — check if JAR asset is already there
    if gh release view "$TAG" --repo "$REPO" --json assets --jq '.assets[].name' 2>/dev/null | grep -q "^${JAR_NAME}$"; then
        echo "JAR already published at $REPO release $TAG — skipping upload."
        echo "To force re-upload, delete the asset first:"
        echo "  gh release delete-asset $TAG $JAR_NAME --repo $REPO --yes"
        exit 0
    fi
    # Release exists but no JAR — upload the asset
    echo "=== Uploading JAR to existing release $TAG ==="
    gh release upload "$TAG" "$JAR_PATH" --repo "$REPO"
else
    # No release — create it with the JAR
    echo "=== Creating release $TAG with JAR ==="
    gh release create "$TAG" "$JAR_PATH" \
        --repo "$REPO" \
        --title "Fixture generator tools $TAG" \
        --notes "Pre-built shadow JAR containing fixture generator, L4 validator, L5 encoder, and L6 fuzz encoder for taps-keys validation workflows."
fi

echo "=== Done ==="
echo "Download URL: https://github.com/$REPO/releases/download/$TAG/$JAR_NAME"
