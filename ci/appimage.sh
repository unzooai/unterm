#!/bin/bash
# Build a Linux AppImage for Unterm.
# Run after: cargo build --release -p unterm -p unterm-cli -p unterm-mux -p strip-ansi-escapes
set -euo pipefail
set -x

rm -rf AppDir *.AppImage *.zsync
mkdir AppDir

install -Dsm755 -t AppDir/usr/bin target/release/unterm
install -Dsm755 -t AppDir/usr/bin target/release/unterm-cli
install -Dsm755 -t AppDir/usr/bin target/release/unterm-mux
install -Dsm755 -t AppDir/usr/bin target/release/strip-ansi-escapes
install -Dm644 assets/icon/terminal.png AppDir/usr/share/icons/hicolor/128x128/apps/ai.unzoo.unterm.png
install -Dm644 assets/icon/unterm-icon.svg AppDir/usr/share/icons/hicolor/scalable/apps/ai.unzoo.unterm.svg
install -Dm644 assets/unterm.desktop AppDir/usr/share/applications/ai.unzoo.unterm.desktop
install -Dm644 assets/unterm.appdata.xml AppDir/usr/share/metainfo/ai.unzoo.unterm.appdata.xml

[ -x /tmp/linuxdeploy ] || ( curl -L 'https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage' -o /tmp/linuxdeploy && chmod +x /tmp/linuxdeploy )

TAG_NAME=${TAG_NAME:-$(git -c "core.abbrev=8" show -s "--format=%cd-%h" "--date=format:%Y%m%d-%H%M%S")}
OUTPUT=Unterm-$TAG_NAME-x86_64.AppImage

VERSION="$TAG_NAME" \
UPDATE_INFORMATION="gh-releases-zsync|unzooai|unterm|latest|Unterm-*.AppImage.zsync" \
OUTPUT="$OUTPUT" \
  /tmp/linuxdeploy \
  --exclude-library='libwayland-client.so.0' \
  --appdir AppDir \
  --output appimage \
  --desktop-file assets/unterm.desktop
