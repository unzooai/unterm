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

# Linux 128x128 — kept as-is (designer-exported original).
# If you ever need to regenerate it from the 512 source instead, run:
#   magick unterm-icon-512.png -resize 128x128 terminal.png

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

# Windows .ico — kept as-is (designer-exported original).
# If you need a multi-resolution build (16/32/48/64/96/128/256) instead, run:
#   magick unterm-icon-512.png -define icon:auto-resize=256,128,96,64,48,32,16 ../windows/terminal.ico
