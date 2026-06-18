#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "capture-macos-debug-window.sh is macOS-only" >&2
  exit 2
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="${1:-/tmp/unterm-debug-window.png}"
debug_home="$(mktemp -d /tmp/unterm-ui-home.XXXXXX)"
pid=""

cleanup() {
  if [[ -n "${pid}" ]] && kill -0 "${pid}" 2>/dev/null; then
    kill "${pid}" 2>/dev/null || true
    wait "${pid}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

mkdir -p "${debug_home}/Desktop" "${debug_home}/.local/share/unterm"

pushd "${root}" >/dev/null
HOME="${debug_home}" RUST_BACKTRACE=1 \
  target/debug/unterm start --always-new-process --cwd /tmp \
  > /tmp/unterm-debug-window.log 2>&1 &
pid="$!"
popd >/dev/null

sleep "${UNTERM_CAPTURE_WAIT_SECONDS:-8}"

window_id="$(
  PID="${pid}" swift - <<'SWIFT'
import CoreGraphics
import Foundation

let target = Int(ProcessInfo.processInfo.environment["PID"] ?? "0") ?? 0
let list = CGWindowListCopyWindowInfo([.optionAll, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
var best: (id: Int, area: Int)?

for window in list {
    guard (window[kCGWindowOwnerPID as String] as? Int ?? 0) == target else { continue }
    let id = window[kCGWindowNumber as String] as? Int ?? 0
    let bounds = window[kCGWindowBounds as String] as? [String: Any] ?? [:]
    let width = bounds["Width"] as? Int ?? 0
    let height = bounds["Height"] as? Int ?? 0
    let area = width * height
    if area > (best?.area ?? 0) {
        best = (id, area)
    }
}

if let best {
    print(best.id)
}
SWIFT
)"

if [[ -z "${window_id}" ]]; then
  echo "no Unterm window found for pid ${pid}" >&2
  tail -80 /tmp/unterm-debug-window.log >&2 || true
  exit 1
fi

screencapture -x -l"${window_id}" "${out}"
echo "${out}"
