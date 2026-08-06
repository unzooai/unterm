#!/bin/bash
# Build a Linux AppImage for Unterm.
# Run after: cargo build --release -p unterm-app -p unterm-cli -p unterm-core
set -euo pipefail
set -x

rm -rf AppDir *.AppImage *.zsync
mkdir AppDir

install -Dsm755 -t AppDir/usr/bin target/release/unterm
install -Dsm755 -t AppDir/usr/bin target/release/unterm-cli
install -Dsm755 -t AppDir/usr/bin target/release/unterm-core
install -Dm644 assets/unterm.conf AppDir/usr/bin/unterm.conf
install -Dm644 assets/fonts/SymbolsNerdFontMono-Regular.ttf \
  AppDir/usr/share/unterm/fonts/SymbolsNerdFontMono-Regular.ttf
install -Dm644 assets/fonts/NotoColorEmoji.ttf \
  AppDir/usr/share/unterm/fonts/NotoColorEmoji.ttf
for s in 16 24 32 48 64 96 128 256 512 ; do
  install -Dm644 "assets/icon/hicolor/${s}x${s}/ai.unzoo.unterm.png" \
    "AppDir/usr/share/icons/hicolor/${s}x${s}/apps/ai.unzoo.unterm.png"
done
install -Dm644 assets/icon/unterm-icon.svg AppDir/usr/share/icons/hicolor/scalable/apps/ai.unzoo.unterm.svg
install -Dm644 assets/unterm.desktop AppDir/usr/share/applications/ai.unzoo.unterm.desktop
install -Dm644 assets/unterm.appdata.xml AppDir/usr/share/metainfo/ai.unzoo.unterm.appdata.xml

# Arch follows the build host (native runner): x86_64 or aarch64. linuxdeploy
# ships a per-arch AppImage; the output filename carries the arch so x64 and
# arm64 release artifacts don't collide.
ARCH=$(uname -m)
[ -x /tmp/linuxdeploy ] || ( curl -L "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-${ARCH}.AppImage" -o /tmp/linuxdeploy && chmod +x /tmp/linuxdeploy )

TAG_NAME=${TAG_NAME:-$(git -c "core.abbrev=8" show -s "--format=%cd-%h" "--date=format:%Y%m%d-%H%M%S")}
OUTPUT=Unterm-$TAG_NAME-$ARCH.AppImage

VERSION="$TAG_NAME" \
UPDATE_INFORMATION="gh-releases-zsync|zhitongblog|unterm|latest|Unterm-*.AppImage.zsync" \
OUTPUT="$OUTPUT" \
  /tmp/linuxdeploy \
  --exclude-library='libwayland-client.so.0' \
  --appdir AppDir \
  --output appimage \
  --desktop-file assets/unterm.desktop
