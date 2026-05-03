# Windows-side handoff — 2026-05-03

This file is the live handoff between the macOS development session and
whoever picks up next (you on Windows, possibly with a fresh Claude
Code session). Delete this file once the issues below are resolved
and shipped.

---

## TL;DR

`v0.11` is the latest tag. Five-platform release is live:
https://github.com/unzooai/unterm/releases/tag/v0.11

**Two issues still reported on Windows after v0.11**, so v0.11 did not
fully solve them:

1. **White flash on launch** — still appears, briefly. v0.10 added a
   `FillRect(BLACK_BRUSH)` to `WM_ERASEBKGND`; v0.11 made it fire only
   on the first invocation per window. User says the flash is still
   visible, which suggests the timing is still wrong — the window is
   becoming visible *before* our fill runs.

2. **Slight UI lag, slow startup** — v0.10 was definitely slow because
   `WM_ERASEBKGND` did a full FillRect on every fire. v0.11's per-window
   `did_initial_erase` flag should eliminate the per-fire cost. If lag
   is still there in v0.11, the cause is somewhere else — start the
   investigation fresh.

---

## Current state of the relevant code

`window/src/os/windows/window.rs`:

- WNDCLASSW.hbrBackground = `BLACK_BRUSH` (line ~434). Harmless because
  our WindowProc returns 1 from WM_ERASEBKGND so the brush is never
  actually used by DefWindowProc. Could revert to `null_mut()` with no
  visible difference.

- WindowProc handles `WM_ERASEBKGND` (line ~2976):
  - `try_borrow_mut` on the inner WindowInner.
  - If `did_initial_erase == false`: `GetClientRect` + `FillRect(hdc,
    rect, BLACK_BRUSH)`, then set the flag.
  - Else: just `Some(1)`.

- `WindowInner` has `did_initial_erase: bool` field, defaulted to false
  at construction.

- Window geometry on Windows: query `MONITORINFO.rcWork`, clamp width/
  height to `(work_area − 16px)`, center in work area, drop the old
  `CW_USEDEFAULT` path (line ~453). New code path covers ALL
  non-WS_POPUP launches, not just popups like the previous code.

## Leading hypothesis for the residual white flash

**The window is visible before WM_ERASEBKGND fires.**

Sequence:

1. `CreateWindowExW` returns. Window is hidden (no WS_VISIBLE).
2. `WindowInner::show()` calls `schedule_show_window` which `spawn`s
   an async task that eventually calls `ShowWindow(SW_NORMAL)`.
3. Windows DWM allocates a redirection bitmap with default contents
   (white) and shows the window.
4. **At some point** Windows posts WM_ERASEBKGND to our WindowProc.
   By the time we paint black, the user has already seen ~1 frame of
   white redirection bitmap.

Steps to confirm: instrument WM_NCCREATE / WM_CREATE / WM_PAINT /
WM_ERASEBKGND / WM_SHOWWINDOW with `log::error!` timestamps in
`do_wnd_proc`, plus log timestamps before and after `ShowWindow` in
`schedule_show_window`. Run unterm with `RUST_LOG=error` redirected to
a file and look at the ordering.

## The fix to try

Move the black fill out of WM_ERASEBKGND and into a synchronous GDI
paint *before* ShowWindow makes the window visible. The WM_ERASEBKGND
handler can then revert to plain `Some(1)`.

Sketch (in `window/src/os/windows/window.rs`):

```rust
fn show(&self) {
    // Synchronously paint the client area black BEFORE asking Windows
    // to show the window. WM_ERASEBKGND fires async via the message
    // pump and is not guaranteed to run before DWM presents the
    // initial frame, which is why our previous fix in v0.10/v0.11
    // didn't fully eliminate the flash.
    unsafe {
        let hdc = GetDC(self.0);
        if !hdc.is_null() {
            let mut rect: RECT = std::mem::zeroed();
            GetClientRect(self.0, &mut rect);
            let brush = GetStockObject(BLACK_BRUSH as i32) as HBRUSH;
            if !brush.is_null() {
                FillRect(hdc, &rect, brush);
            }
            ReleaseDC(self.0, hdc);
        }
    }
    schedule_show_window(self.0, ShowWindowCommand::Normal);
}
```

After confirming this works, you can remove the WM_ERASEBKGND
`did_initial_erase` flag entirely and revert that handler to the
original `Some(1)` — the synchronous paint covers the only moment
that mattered (first show), and steady-state ERASEBKGND is now a
pure no-op like upstream WezTerm.

## If lag persists after the white-flash fix

Don't blame WM_ERASEBKGND. Look at:

- **Shader compile** on first launch: WezTerm-on-Windows uses ANGLE
  (DirectX → OpenGL translation). First-launch shader compile can
  take seconds. `~/.unterm/cache/` may help if it persists between
  runs; check what's getting cached.
- **Multi-instance scan at startup**: `server_info.rs` runs
  `live_instances_locked` which reads the `instances/` dir and
  PID-checks each entry. With many stale instance files on disk the
  scan could noticeable. Run `Get-ChildItem $HOME\.unterm\instances\`
  to count files; if >100 there's a bug in cleanup.
- **Proxy auto-detection**: `system_proxy::detect()` on Windows reads
  the IE registry keys and probes ports. Synchronous on the main
  thread during pane spawn. Add timing logs around it.
- **ConPTY initialization**: spawning the first PowerShell shell.
  Windows ConPTY has its own latency; user-perceived lag may be
  shell startup, not unterm.

## Things known good (don't accidentally break)

- Multi-instance NATO discovery works on Linux (verified via OrbStack
  Xvfb on Mac) and macOS. Don't touch `server_info.rs` semantics
  unless you're sure.
- Cargo.lock has been regenerated for v0.11. Any `cargo update`
  without `-p <our crates>` will accidentally bump unrelated
  transitive deps.
- `core-foundation = "=0.11.0"` in workspace deps is intentionally
  pinned with `=`. If you bump our crates to v0.12, do NOT let your
  perl regex hit `0.11.0` in this line.
- The `release-{linux,windows}.yml` workflows publish via
  `tag_name: ${{ steps.tag.outputs.name }}` so push-tag AND
  workflow_dispatch both work. Don't reintroduce the
  `if: startsWith(github.ref, 'refs/tags/')` guard.

## Release-cadence rule (from `~/.claude/.../memory/feedback_release_cadence.md`)

- Patches accumulate on master. Don't tag every commit.
- Only tag minor versions (`vX.Y`, two-segment) when the user
  explicitly says "release".
- The `release-*.yml` workflows trigger on tags matching
  `v[0-9]+.[0-9]+` only — that filter excludes patch tags.

## Where to put your changes

Just push to `master`. CI runs on every push (lint/build), but the
release workflows only fire on minor tags. Tag a new `v0.12` once
both the white flash and the lag are confirmed fixed; don't ship
intermediate v0.11.x patches.

## Useful one-liners

```sh
# build for local Windows test, just unterm-gui
cargo build --release -p unterm

# run the freshly-built binary with debug logging to a file
.\target\release\unterm.exe 2>&1 | Tee-Object .\startup.log

# trigger release for a minor tag (only after verifying everything works)
git tag -a v0.12 -m "Unterm 0.12.0 — white flash + lag final fix"
git push origin v0.12
# CI auto-fires Linux + Windows; Mac DMG: ssh to mac and `make release-mac`
```

## Cross-machine memory note

Claude memory at `~/.claude/projects/-Volumes-Dev-code-unterm/memory/`
is local to the Mac. Notable memories that should be re-read on
Windows side if Claude Code is used:

- `feedback_release_cadence.md` — minor tags only, don't spam patches.
- `feedback_self_debug_via_mcp.md` — self-test via MCP/CLI, not GUI.
- `feedback_subtraction_principle.md` — delete awkward features rather
  than redesign.
- `project_dogfood_milestone.md` — Unterm is the user's daily terminal;
  regressions block work.
- `project_multi_instance_design.md` — NATO names, auto-title,
  active.json on prev-death only.
- `project_positioning.md` — "the terminal AI agents can drive."

If continuing in Claude Code on Windows, reach into the equivalent
`~/.claude` path and re-create these notes from this repo's history,
or read this HANDOFF.md as the source of truth for the in-progress
issues.
