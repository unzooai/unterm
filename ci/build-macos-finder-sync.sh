#!/bin/bash
# Build the Finder Sync extension and place it inside Unterm.app.
#
# Usage:
#   ci/build-macos-finder-sync.sh Unterm.app
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "usage: $0 /path/to/Unterm.app" >&2
  exit 2
fi

APP="$1"
ROOT=$(git rev-parse --show-toplevel)
SRC_DIR="$ROOT/assets/macos/FinderSync"
BUILD_DIR="$ROOT/target/macos-finder-sync"
APPEX="$APP/Contents/PlugIns/UntermFinderSync.appex"
EXECUTABLE="UntermFinderSync"
DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-11.0}"

rm -rf "$BUILD_DIR" "$APPEX"
mkdir -p "$BUILD_DIR" "$APPEX/Contents/MacOS"
cp "$SRC_DIR/Info.plist" "$APPEX/Contents/Info.plist"
if [ -n "${TAG_NAME:-}" ]; then
  VERSION="${TAG_NAME#v}"
  /usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" "$APPEX/Contents/Info.plist"
fi

for arch in arm64 x86_64; do
  sdk=$(xcrun --sdk macosx --show-sdk-path)
  swiftc \
    -sdk "$sdk" \
    -target "$arch-apple-macosx$DEPLOYMENT_TARGET" \
    -module-name UntermFinderSync \
    -parse-as-library \
    -framework Cocoa \
    -framework FinderSync \
    -Xlinker -e \
    -Xlinker _NSExtensionMain \
    "$SRC_DIR/FinderSyncExtension.swift" \
    -o "$BUILD_DIR/$EXECUTABLE-$arch"
done

lipo -create \
  "$BUILD_DIR/$EXECUTABLE-arm64" \
  "$BUILD_DIR/$EXECUTABLE-x86_64" \
  -output "$APPEX/Contents/MacOS/$EXECUTABLE"

chmod +x "$APPEX/Contents/MacOS/$EXECUTABLE"
