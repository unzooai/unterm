#!/bin/bash
# Regenerate platform icons from the source 512x512 PNG (or SVG fallback).
# Run from any cwd. Output:
#   assets/icon/terminal.png                                  — 128x128 (Linux .desktop legacy path)
#   assets/icon/hicolor/<size>x<size>/ai.unzoo.unterm.png     — Linux hicolor theme PNGs
#   assets/macos/Unterm.app/Contents/Resources/terminal.icns  — macOS bundle icon
#   assets/windows/terminal.ico                               — Windows .exe + MSI shortcut
set -euo pipefail
set -x

cd "$(git rev-parse --show-toplevel)/assets/icon"

SRC_PNG=unterm-icon-512.png
# Small-size optical variant: thicker stems + larger cursor so the mark
# stays legible at Dock/taskbar/favicon sizes. Used for all sizes <= 64.
SRC_PNG_SMALL=unterm-icon-small-512.png
SRC_SVG=unterm-icon.svg

# Dependency check
have() { command -v "$1" >/dev/null 2>&1; }

resize_to() {
  local size=$1 out=$2
  local src="$SRC_PNG"
  if [ "$size" -le 64 ] && [ -f "$SRC_PNG_SMALL" ]; then
    src="$SRC_PNG_SMALL"
  fi
  if have magick ; then
    magick "$src" -resize "${size}x${size}" "$out"
  elif have convert ; then
    convert "$src" -resize "${size}x${size}" "$out"
  elif have sips ; then
    cp "$src" "$out" && sips -Z "$size" "$out" >/dev/null
  else
    echo "ERROR: need 'magick', 'convert', or 'sips' to resize PNG" >&2
    exit 1
  fi
}

# Linux .desktop / .deb icon — primary 128x128 + the @2x fallback are kept for
# backward compat with older packaging scripts that reference these paths.
resize_to 128 terminal.png
resize_to 256 terminal@2x.png

# Website/favicon assets share the same master and small-size optical rules.
# Keeping them in this pipeline prevents the installed app and product site
# from drifting into different brand marks after a future icon refresh.
cp "$SRC_SVG" ../../web/public/assets/icon.svg
for s in 32 256 512 ; do
  resize_to "$s" "../../web/public/assets/icon-${s}.png"
done

# Linux hicolor ladder — install one PNG per standard icon size so the desktop
# environment can pick a crisp raster at every spot (taskbar=16/24, app
# launcher=48/64, file dialog=96/128, Activities/grid=256/512). Without this,
# DEs end up scaling the 128 PNG or — worse — falling back to the SVG, which
# renders our font-dependent text glyphs with whatever monospace the system
# has (often the wrong shape). PNGs are font-independent rasters, so this is
# the safest cross-distro path.
rm -rf hicolor
for s in 16 24 32 48 64 96 128 256 512 ; do
  out="hicolor/${s}x${s}/ai.unzoo.unterm.png"
  mkdir -p "$(dirname "$out")"
  resize_to "$s" "$out"
done

# macOS .icns
ICONSET=$(mktemp -d)/Unterm.iconset
mkdir -p "$ICONSET"
for s in 16 32 64 128 256 512 ; do
  out="$ICONSET/icon_${s}x${s}.png"
  resize_to "$s" "$out"
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
if have magick || have convert ; then
  WIN_ICONSET=$(mktemp -d)
  frames=()
  for s in 256 128 96 64 48 32 24 16 ; do
    frame="$WIN_ICONSET/icon-${s}.png"
    resize_to "$s" "$frame"
    frames+=("$frame")
  done
  if have magick ; then
    magick "${frames[@]}" ../windows/terminal.ico
  else
    convert "${frames[@]}" ../windows/terminal.ico
  fi
  rm -rf "$WIN_ICONSET"
fi
