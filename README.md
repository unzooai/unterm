# Unterm

**The terminal AI agents can drive.**

Cross-platform terminal (macOS / Linux / Windows) built on a customized WezTerm engine, with one design bet: the terminal itself is controllable from the outside by any AI agent over MCP. Claude Code, Cursor, Aider, Continue, your own scripts — they all get the same JSON-RPC surface to spawn shells, run commands, read pane state, capture screenshots, change settings, and record sessions.

The other 2026 terminals each pick a different side: Warp embeds AI inside a closed cloud (Oz), Ghostty stays out of your way and lets you bring your own tools, iTerm2 is Mac-only. Unterm picks the third side — terminal as MCP-controllable surface, deliberately keep AI features *out* of the terminal, let external agents grip it through the API.

Practical implications:

- Every Unterm window starts a local **MCP server** (line-delimited JSON-RPC over TCP) and a local **HTTP settings server** (Web Settings page) on auto-allocated ports. Both are auth-token gated, both are 127.0.0.1-only, no cloud round trip.
- **Settings live in the browser**, not the terminal. Cell-grid TUIs can't deliver modern form UX (no proper text inputs, no live preview, no color picker). The in-terminal `▼` overlay is intentionally minimal — five quick actions and a link to the Web Settings page.
- **9 languages out of the box**: en / 简体中文 / 繁體中文 / 日本語 / 한국어 / Deutsch / Français / Italiano / हिन्दी. Auto-detects from system locale, can be overridden in Web Settings or via `unterm-cli lang set <code>`.
- **Cross-platform parity is a correctness property**: if a feature works on Windows but bails on macOS or Linux, that's a bug, not "not supported yet."
- **Subtraction over decoration**: no AI overlay inside the terminal, no inline image render that wedges the GUI, no right-click menu, no Cmd+Q confirmation, no manual proxy URL config (auto-detected from system).

Built on top of the WezTerm engine for renderer / font / TUI / SSH / mux work, with a thin product layer on top.

---

## Install

Pre-built artifacts are published on GitHub Releases:

https://github.com/unzooai/unterm/releases

| Platform | Artifact                                                    |
| -------- | ----------------------------------------------------------- |
| macOS    | `Unterm-macos-<version>.zip` (universal arm64+x86_64)       |
| Linux    | `unterm-<version>.deb` or `Unterm-<version>-x86_64.AppImage` |
| Windows  | `Unterm-<version>-x64.msi` or `Unterm-windows-<version>.zip` |

### macOS

```bash
unzip Unterm-macos-<version>.zip
mv Unterm-macos-<version>/Unterm.app /Applications/
```

### Linux (Debian / Ubuntu)

```bash
sudo apt install ./unterm-<version>.deb
unterm
```

Other distros — use the AppImage:

```bash
chmod +x Unterm-<version>-x86_64.AppImage
./Unterm-<version>-x86_64.AppImage
```

### Windows

Run the MSI installer; it places `unterm.exe` in `Program Files\Unterm` and creates a Start Menu shortcut.

---

## Features

- **GPU-accelerated rendering** on all three platforms (Metal / OpenGL / DirectX via ANGLE).
- **MCP server** on `127.0.0.1:<auto-port>` (default 19876) — JSON-RPC over TCP, auth-token gated. Methods cover sessions, exec, screen reads, signal, orchestrate, proxy, theme, capture, recording, policy, system, server.
- **Web Settings UI** on `127.0.0.1:<auto-port>` (default 19877) — open in any browser via `unterm-cli settings open` or the `Settings (Web)` item in the `▼` menu. Tailwind-styled SPA, supports all 9 languages, keyboard + mouse.
- **Auto proxy detection** — reads macOS System Preferences / Windows registry / GNOME gsettings / `$HTTPS_PROXY`, falls back to scanning common local ports. The single `proxy.json` toggle is `{"enabled": true|false}` — no manual URL configuration needed.
- **Region screenshots** from the status bar (left-click excludes the Unterm window, right-click includes it). PNG lands on disk under `~/.unterm/screenshots/`, on the system image clipboard, and the path on the text clipboard.
- **Session recording → markdown** with OSC 133 block segmentation and built-in redaction (GitHub tokens / `KEY=value` / 40+ char hex/base64 patterns are masked). Recordings are stored in the project directory under `<cwd>/.unterm/sessions/<date>/<tab>-<time>.md`, or in `~/.unterm/sessions/_orphan/` when no writable project context.
- **Right-click is a direct gesture, not a menu**: with a selection it copies and clears; without selection it pastes.
- **Slim quick-action overlay** on the tab bar's `▼` button:
  - Change Working Directory (cd current pane)
  - Open Folder in New Tab
  - Split Right (left/right pane split)
  - Toggle Session Recording
  - Export Current Session
  - Settings (Web)
- **macOS-native window decorations** (traffic-light buttons + native title bar); Windows uses Windows Terminal-style integrated title buttons; Linux uses client-side decorations.

---

## CLI

The `unterm-cli` binary exposes the full Unterm product surface, transparently routing to the local MCP server. Read `~/.unterm/server.json` for current ports + auth.

```bash
# Settings + Web UI
unterm-cli settings open                       # open the Web Settings page
unterm-cli theme list / set <id>               # standard / midnight / daylight / classic
unterm-cli lang list / set <code> / current    # en-US / zh-CN / zh-TW / ja-JP / ko-KR / de-DE / fr-FR / it-IT / hi-IN

# Proxy
unterm-cli proxy status                        # auto-detect health
unterm-cli proxy nodes / switch <name> / disable / env

# Sessions / panes
unterm-cli session list                        # list panes
unterm-cli session record start [--id N]
unterm-cli session record stop [--id N]
unterm-cli session export [--id N] [-o FILE]
unterm-cli sessions list [--project SLUG]
unterm-cli sessions read <session-id>

# Screenshots
unterm-cli screenshot [--include-window] [-o FILE]
```

Pass `--json` to any subcommand for raw JSON-RPC output (suitable for scripts).

---

## Configuration

User config lives at:

| Platform | Location                                 |
| -------- | ---------------------------------------- |
| macOS    | `~/.unterm/`                             |
| Linux    | `~/.unterm/`                             |
| Windows  | `%USERPROFILE%\.unterm\`                 |

Files:

| File              | Purpose                                          |
| ----------------- | ------------------------------------------------ |
| `server.json`     | Current MCP/HTTP ports + auth token + pid (auto, rewritten on launch) |
| `auth_token`      | Legacy mirror of the auth token (for back-compat) |
| `proxy.json`      | `{"enabled": true|false}` — URLs auto-detected   |
| `theme.json`      | Active theme id                                  |
| `lang.json`       | Persisted locale override                        |
| `recording.json`  | Recording config (redaction patterns, etc.)      |
| `sessions/`       | Recording metadata index (per-project subdirs)   |
| `screenshots/`    | Region screenshots (PNG)                         |

---

## Development

Prereqs: a recent stable Rust toolchain. Linux additionally needs the system deps in `get-deps`.

```bash
make build        # all binaries (debug)
make check        # static checks
make test         # tests
```

Build a release for the current platform:

```bash
cargo build --release -p unterm -p unterm-cli -p unterm-mux -p strip-ansi-escapes
```

Build platform packages:

```bash
# macOS — universal .app + zip (run on macOS)
ci/deploy.sh

# Linux — .deb
ci/deploy.sh
# Linux — AppImage
ci/appimage.sh

# Windows — staged release tree + zip
bash ci/deploy.sh
# Windows — MSI (requires WiX 4 at .\.tools\wix.exe)
pwsh -File ci/build-msi.ps1
```

macOS code-signing + notarization is **local-only** (no CI step) so the
Developer ID `.p12` private key never has to leave your Mac. One-time
setup, on the Mac that holds the cert:

```bash
xcrun notarytool store-credentials UntermNotary \
  --apple-id <your-apple-id> --team-id 6NQM3XP5RF
```

Then for every release:

```bash
git tag -a vX.Y.Z -m "Unterm vX.Y.Z" && git push origin vX.Y.Z
make release-mac                    # build universal + sign + notarize + upload
```

`make release-mac` reads the tag from `git describe --exact-match HEAD`,
builds universal x86_64+aarch64 binaries, calls `ci/sign-macos.sh` with
`NOTARY_PROFILE=UntermNotary`, then `gh release upload`s the resulting
zip to the matching GitHub Release.

CI on every PR runs `cargo check` against macOS, Linux, and Windows.
Tagged pushes (`vX.Y.Z`) trigger the `release-linux` and
`release-windows` workflows that publish those two platforms' artifacts
to GitHub Releases. macOS sits out of CI by design — see above.

---

## Repository

This repository is the main Unterm project:

https://github.com/unzooai/unterm

Unterm includes modified WezTerm components. Upstream WezTerm remains a separate project by Wez Furlong and contributors.
