#!/bin/bash
# Sign and (optionally) notarize a local macOS Unterm.app build.
#
# Usage:
#   ci/sign-macos.sh                          # sign with default Developer ID, no notarize
#   NOTARY_PROFILE=UntermNotary ci/sign-macos.sh
#
# To create the notary profile once (interactive — you'll be asked for Apple ID
# and an app-specific password from https://account.apple.com/account/manage):
#   xcrun notarytool store-credentials UntermNotary \
#     --apple-id <your-apple-id> \
#     --team-id 6NQM3XP5RF
set -euo pipefail
set -x

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

TARGET_DIR=${TARGET_DIR:-target}
TAG_NAME=${TAG_NAME:-local-$(date +%Y%m%d-%H%M%S)}
DEV_ID=${DEV_ID:-"Developer ID Application: xiangdong li (6NQM3XP5RF)"}
NOTARY_PROFILE=${NOTARY_PROFILE:-}

# Stage the .app
zipdir=Unterm-macos-$TAG_NAME
zipname=$zipdir.zip
rm -rf "$zipdir" "$zipname"
mkdir "$zipdir"
cp -r assets/macos/Unterm.app "$zipdir/"
rm -f "$zipdir/Unterm.app/"*.dylib
mkdir -p "$zipdir/Unterm.app/Contents/MacOS"
mkdir -p "$zipdir/Unterm.app/Contents/Resources"
cp -r assets/shell-integration/* "$zipdir/Unterm.app/Contents/Resources"
cp -r assets/shell-completion "$zipdir/Unterm.app/Contents/Resources"
tic -xe wezterm -o "$zipdir/Unterm.app/Contents/Resources/terminfo" termwiz/data/wezterm.terminfo

for bin in unterm unterm-cli unterm-mux strip-ansi-escapes ; do
  if [[ -f "$TARGET_DIR/release/$bin" ]] ; then
    cp "$TARGET_DIR/release/$bin" "$zipdir/Unterm.app/Contents/MacOS/$bin"
  elif compgen -G "$TARGET_DIR/*/release/$bin" >/dev/null ; then
    lipo "$TARGET_DIR"/*/release/$bin -output "$zipdir/Unterm.app/Contents/MacOS/$bin" -create
  else
    echo "ERROR: missing build artifact $bin — run 'cargo build --release -p unterm -p unterm-cli -p unterm-mux -p strip-ansi-escapes' first"
    exit 1
  fi
done

# Sign every binary individually then the bundle (deep)
for bin in "$zipdir/Unterm.app/Contents/MacOS/"* ; do
  /usr/bin/codesign --force --options runtime --timestamp \
    --entitlements ci/macos-entitlement.plist \
    --sign "$DEV_ID" "$bin"
done
/usr/bin/codesign --force --options runtime --timestamp \
  --entitlements ci/macos-entitlement.plist \
  --sign "$DEV_ID" "$zipdir/Unterm.app"

/usr/bin/codesign --verify --strict --verbose=2 "$zipdir/Unterm.app"

# Zip up for distribution / notarization
ditto -c -k --keepParent "$zipdir/Unterm.app" "$zipname"

if [ -n "$NOTARY_PROFILE" ] ; then
  echo "Submitting to Apple notary service via profile ${NOTARY_PROFILE}..."
  xcrun notarytool submit "$zipname" \
    --keychain-profile "$NOTARY_PROFILE" \
    --wait
  # Staple the ticket onto the .app then re-zip so the staple is included
  xcrun stapler staple "$zipdir/Unterm.app"
  rm -f "$zipname"
  ditto -c -k --keepParent "$zipdir/Unterm.app" "$zipname"
  spctl --assess --type execute --verbose "$zipdir/Unterm.app"
fi

set +x
echo "Signed: $zipdir/Unterm.app"
echo "Zip:    $zipname"
# Use a real `if` (not `&& echo`) so the exit status of this script is always
# 0 on success — under `set -e`, a `[ -z "$X" ] && echo` short-circuit returns
# non-zero when $X is set, which would tank any caller that pipefails on us.
if [ -z "$NOTARY_PROFILE" ]; then
  echo "NOTE: not notarized — set NOTARY_PROFILE=<name> after running 'xcrun notarytool store-credentials <name>'"
fi
