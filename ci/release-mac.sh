#!/bin/bash
# Build, sign, notarize, and upload the macOS Unterm.app for the tag at HEAD.
#
# One-time setup (per machine):
#
#   1. Make sure your Developer ID Application cert is in the Mac Keychain
#      (Apple's developer portal → Certificates → Developer ID Application).
#
#   2. Stash an app-specific password for notarization in the Keychain:
#
#        xcrun notarytool store-credentials UntermNotary \
#          --apple-id <your-apple-id> \
#          --team-id 6NQM3XP5RF
#
#      (App-specific passwords are generated at
#       https://account.apple.com/account/manage → Sign-In and Security →
#       App-Specific Passwords.)
#
# Usage:
#
#   make release-mac                                 # uses NOTARY_PROFILE=UntermNotary
#   make release-mac NOTARY_PROFILE=OtherProfile     # different stored profile
#
# What it does:
#
#   - Confirms HEAD has an annotated tag.
#   - Builds release universal (x86_64 + aarch64) for unterm/unterm-cli/unterm-mux.
#   - Calls ci/sign-macos.sh which signs, notarizes, staples, and zips the .app.
#   - Uploads the resulting zip onto the matching GitHub Release.
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

NOTARY_PROFILE=${NOTARY_PROFILE:-UntermNotary}

TAG=$(git describe --tags --exact-match HEAD 2>/dev/null || true)
if [ -z "$TAG" ]; then
  echo "ERROR: HEAD has no tag. Tag a release first:" >&2
  echo "  git tag -a vX.Y.Z -m 'Unterm vX.Y.Z' && git push origin vX.Y.Z" >&2
  exit 1
fi

if ! xcrun notarytool history --keychain-profile "$NOTARY_PROFILE" >/dev/null 2>&1; then
  echo "ERROR: Notary profile '$NOTARY_PROFILE' not found in Keychain." >&2
  echo "Run once:" >&2
  echo "  xcrun notarytool store-credentials $NOTARY_PROFILE \\" >&2
  echo "    --apple-id <your-apple-id> --team-id 6NQM3XP5RF" >&2
  exit 1
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "ERROR: 'gh' CLI not found. Install with: brew install gh" >&2
  exit 1
fi

echo ">> Building universal release for $TAG"
rustup target add x86_64-apple-darwin aarch64-apple-darwin >/dev/null
for triple in x86_64-apple-darwin aarch64-apple-darwin; do
  cargo build --release --target "$triple" \
    -p unterm -p unterm-cli -p unterm-mux -p strip-ansi-escapes
done

echo ">> Signing + notarizing as $TAG"
TAG_NAME="$TAG" NOTARY_PROFILE="$NOTARY_PROFILE" bash ci/sign-macos.sh

dmg="Unterm-macos-$TAG.dmg"
if [ ! -f "$dmg" ]; then
  echo "ERROR: expected $dmg not produced by ci/sign-macos.sh" >&2
  exit 1
fi

echo ">> Uploading $dmg to release $TAG"
# `--clobber` so re-runs just overwrite the asset; `gh release create` first if
# the release doesn't exist yet (the Linux/Windows workflows usually create it).
if ! gh release view "$TAG" >/dev/null 2>&1; then
  gh release create "$TAG" --title "Unterm $TAG" --notes "Unterm $TAG"
fi
gh release upload "$TAG" "$dmg" --clobber

echo ">> Done. Asset $dmg attached to release $TAG."
