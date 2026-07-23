# Agent Cockpit demo recording kit

Source for `assets/demo/agent-cockpit.gif` and `web/public/assets/demo-cockpit.mp4`.

Three fake agents impersonate Claude Code / Codex / Gemini well enough for
every cockpit layer to treat them as real:

- `demo_claude.sh` / `demo_codex.sh` / `demo_gemini.sh` — self-timed pane
  scripts. Codex hits its approval prompt (OSC 9 `approval-requested` + BEL)
  22s after launch; answering `y` completes the turn (OSC 9
  `agent-turn-complete`).
- Run each via a bash copy named after the agent so the process-fingerprint
  layer classifies it: `mkdir /tmp/agentbin && for a in claude codex gemini;
  do cp /bin/bash /tmp/agentbin/$a && codesign -f -s - /tmp/agentbin/$a; done`
  (the ad-hoc codesign is required — copies of SIP-protected binaries are
  killed on exec otherwise).

Hard-won constraints for the choreography (synthetic input via cliclick):

1. Screen must be unlocked, and the machine must be yours for the duration —
   verify the frontmost pid before EVERY injected event and abort otherwise.
2. Switch the input source to ABC first (`TISSelectInputSource`); a CJK IME
   turns typed letters into compositions and eats Enter.
3. cliclick's `kp:enter` does not reach the pane — send Return via
   `osascript -e 'tell application "System Events" to key code 36'`.
4. Don't use `sleep` in the pane scripts: the child process becomes the
   foreground process and breaks agent fingerprinting. `snooze()` reads
   /dev/tty with a timeout instead (childless, truly blocks).
5. Paste is async; type commands with `cliclick t:` (synchronous) or verify
   the text landed before pressing Enter.

Record ~30s with `screencapture -v -V 30 -R<window-rect>`, then:
gif: fps=8, scale=1000, palettegen/paletteuse ·
mp4: scale=1400, libx264 crf26.
