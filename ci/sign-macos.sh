#!/bin/bash
# Sign and (optionally) notarize a local macOS Unterm.app build, then wrap
# it in a signed + notarized .dmg ready for distribution.
#
# Usage:
#   ci/sign-macos.sh                          # sign only, no notarize, no DMG
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

# Stage the .app under Unterm-macos-<tag>/Unterm.app
stagedir=Unterm-macos-$TAG_NAME
dmgname=$stagedir.dmg
rm -rf "$stagedir" "$dmgname"
mkdir "$stagedir"
cp -r assets/macos/Unterm.app "$stagedir/"
rm -f "$stagedir/Unterm.app/"*.dylib
mkdir -p "$stagedir/Unterm.app/Contents/MacOS"
mkdir -p "$stagedir/Unterm.app/Contents/Resources"
cp -r assets/shell-integration/* "$stagedir/Unterm.app/Contents/Resources"
cp -r assets/shell-completion "$stagedir/Unterm.app/Contents/Resources"
tic -xe wezterm -o "$stagedir/Unterm.app/Contents/Resources/terminfo" termwiz/data/wezterm.terminfo

for bin in unterm unterm-cli unterm-mux strip-ansi-escapes ; do
  if [[ -f "$TARGET_DIR/release/$bin" ]] ; then
    cp "$TARGET_DIR/release/$bin" "$stagedir/Unterm.app/Contents/MacOS/$bin"
  elif compgen -G "$TARGET_DIR/*/release/$bin" >/dev/null ; then
    lipo "$TARGET_DIR"/*/release/$bin -output "$stagedir/Unterm.app/Contents/MacOS/$bin" -create
  else
    echo "ERROR: missing build artifact $bin — run 'cargo build --release -p unterm -p unterm-cli -p unterm-mux -p strip-ansi-escapes' first"
    exit 1
  fi
done

# Sign every binary individually then the bundle (deep)
for bin in "$stagedir/Unterm.app/Contents/MacOS/"* ; do
  /usr/bin/codesign --force --options runtime --timestamp \
    --entitlements ci/macos-entitlement.plist \
    --sign "$DEV_ID" "$bin"
done
/usr/bin/codesign --force --options runtime --timestamp \
  --entitlements ci/macos-entitlement.plist \
  --sign "$DEV_ID" "$stagedir/Unterm.app"

/usr/bin/codesign --verify --strict --verbose=2 "$stagedir/Unterm.app"

if [ -n "$NOTARY_PROFILE" ] ; then
  # Notarize the .app first, via a transient zip — Apple's notary service
  # accepts both .zip and .dmg, and zipping the .app is the cheapest container.
  notary_zip="$stagedir.notary.zip"
  rm -f "$notary_zip"
  ditto -c -k --keepParent "$stagedir/Unterm.app" "$notary_zip"
  echo "Submitting .app to Apple notary service via profile ${NOTARY_PROFILE}..."
  xcrun notarytool submit "$notary_zip" \
    --keychain-profile "$NOTARY_PROFILE" \
    --wait
  rm -f "$notary_zip"
  xcrun stapler staple "$stagedir/Unterm.app"
fi

# Build the .dmg. We give it a clean drag-to-install layout: the .app and a
# symlink to /Applications side by side in the mounted volume root.
dmg_stage="$stagedir.dmg-stage"
rm -rf "$dmg_stage"
mkdir "$dmg_stage"
cp -R "$stagedir/Unterm.app" "$dmg_stage/Unterm.app"
ln -s /Applications "$dmg_stage/Applications"
hdiutil create -volname "Unterm" -srcfolder "$dmg_stage" \
  -ov -format UDZO "$dmgname"
rm -rf "$dmg_stage"

# Sign the DMG so Gatekeeper trusts the container itself, not just the .app
# inside. `--timestamp` adds an Apple-server timestamp so verification keeps
# working after the cert eventually expires.
/usr/bin/codesign --force --sign "$DEV_ID" --timestamp "$dmgname"
/usr/bin/codesign --verify --verbose=2 "$dmgname"

if [ -n "$NOTARY_PROFILE" ] ; then
  echo "Submitting .dmg to Apple notary service via profile ${NOTARY_PROFILE}..."
  xcrun notarytool submit "$dmgname" \
    --keychain-profile "$NOTARY_PROFILE" \
    --wait
  xcrun stapler staple "$dmgname"
  spctl --assess --type install --verbose "$dmgname" || true
fi

set +x
echo "Signed: $stagedir/Unterm.app"
echo "DMG:    $dmgname"
# Use a real `if` (not `&& echo`) so the exit status of this script is always
# 0 on success — under `set -e`, a `[ -z "$X" ] && echo` short-circuit returns
# non-zero when $X is set, which would tank any caller that pipefails on us.
if [ -z "$NOTARY_PROFILE" ]; then
  echo "NOTE: not notarized — set NOTARY_PROFILE=<name> after running 'xcrun notarytool store-credentials <name>'"
fi
