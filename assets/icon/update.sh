#!/bin/bash
# Regenerate platform icons from the source 512x512 PNG (or SVG fallback).
# Run from any cwd. Output:
#   assets/icon/terminal.png                                  — 128x128 (Linux .desktop)
#   assets/macos/Unterm.app/Contents/Resources/terminal.icns  — macOS bundle icon
#   assets/windows/terminal.ico                               — Windows .exe + MSI shortcut
set -euo pipefail
set -x

cd "$(git rev-parse --show-toplevel)/assets/icon"

SRC_PNG=unterm-icon-512.png
SRC_SVG=unterm-icon.svg

# Dependency check
have() { command -v "$1" >/dev/null 2>&1; }

# Linux .desktop / .deb icon — 128x128 + 256x256 derived from the UT source.
if have magick ; then
  magick "$SRC_PNG" -resize 128x128 terminal.png
  magick "$SRC_PNG" -resize 256x256 terminal@2x.png
elif have convert ; then
  convert "$SRC_PNG" -resize 128x128 terminal.png
  convert "$SRC_PNG" -resize 256x256 terminal@2x.png
elif have sips ; then
  cp "$SRC_PNG" terminal.png       && sips -Z 128 terminal.png       >/dev/null
  cp "$SRC_PNG" terminal@2x.png    && sips -Z 256 terminal@2x.png    >/dev/null
fi

# macOS .icns
ICONSET=$(mktemp -d)/Unterm.iconset
mkdir -p "$ICONSET"
for s in 16 32 64 128 256 512 ; do
  out="$ICONSET/icon_${s}x${s}.png"
  if have magick ; then
    magick "$SRC_PNG" -resize ${s}x${s} "$out"
  elif have convert ; then
    convert "$SRC_PNG" -resize ${s}x${s} "$out"
  elif have sips ; then
    cp "$SRC_PNG" "$out"
    sips -Z $s "$out" >/dev/null
  fi
  if [[ $s != 16 ]] ; then
    cp "$out" "$ICONSET/icon_$((s/2))x$((s/2))@2x.png"
  fi
done
if have iconutil ; then
  iconutil -c icns -o ../macos/Unterm.app/Contents/Resources/terminal.icns "$ICONSET"
elif have png2icns ; then
  png2icns ../macos/Unterm.app/Contents/Resources/terminal.icns "$ICONSET"/*.png
fi
rm -rf "$ICONSET"

# Windows .ico — multi-resolution embed for crisp rendering at every scale
# Windows actually picks (Start menu = 32, taskbar = 24, desktop = 48, MSI
# launcher = 256, Alt-Tab = 16). Bake in all of them.
if have magick ; then
  magick "$SRC_PNG" -define icon:auto-resize=256,128,96,64,48,32,16 \
    ../windows/terminal.ico
elif have convert ; then
  convert "$SRC_PNG" -define icon:auto-resize=256,128,96,64,48,32,16 \
    ../windows/terminal.ico
fi
