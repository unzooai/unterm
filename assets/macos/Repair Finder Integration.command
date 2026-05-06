#!/bin/bash
set -euo pipefail

APP="/Applications/Unterm.app"
HERE="$(cd "$(dirname "$0")" && pwd)"

if [ ! -d "$APP" ] && [ -d "$HERE/Unterm.app" ]; then
  APP="$HERE/Unterm.app"
fi

if [ ! -d "$APP" ]; then
  osascript -e 'display alert "Unterm is not installed" message "Drag Unterm.app to Applications first, then run this repair tool again."'
  exit 1
fi

LSREGISTER="/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"
APPEX="$APP/Contents/PlugIns/UntermFinderSync.appex"

"$LSREGISTER" -f "$APP"

if [ -d "$APPEX" ]; then
  pluginkit -a "$APPEX" || true
  pluginkit -e use -i ai.unzoo.unterm.finder-sync || true
fi

osascript -e 'display notification "Finder integration has been refreshed." with title "Unterm"'
