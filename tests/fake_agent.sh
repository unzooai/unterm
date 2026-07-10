#!/bin/sh
# Fake CLI agent for cockpit integration testing.
#
# Emits the same OSC signal sequence a real agent produces, with sleeps
# between phases so `unterm-cli agent status --json` can observe each
# state. The process detection layer won't recognize "fake_agent", so
# everything asserted through this script exercises the OSC layer —
# which needs an entry to exist first; the hook layer creates it
# (agent.signal), mirroring how a WSL-hosted agent behaves.
#
# Phases (~3s apart):
#   1. hook signal: working        (creates the registry entry)
#   2. title: braille spinner      → working (osc-title)
#   3. title: ✳ summary            → idle
#   4. OSC 9 notification          → waiting  (approval-requested)
#   5. hook signal: done           → done, decays to idle
#
# Usage: sh tests/fake_agent.sh [unterm-cli-path]

CLI="${1:-unterm-cli}"

esc() { printf '\033]0;%s\007' "$1"; }

"$CLI" agent signal --agent fake --event working
sleep 3

esc "⠼ Fixing the login bug"
sleep 3

esc "✳ Fixing the login bug"
sleep 3

printf '\033]9;approval-requested: run tests?\007'
sleep 3

"$CLI" agent signal --agent fake --event done
sleep 2
