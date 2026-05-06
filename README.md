# Unterm

**The terminal AI agents can drive.**

Cross-platform terminal (macOS / Linux / Windows) built on a customized WezTerm engine, with one design bet: the terminal itself is controllable from the outside by any AI agent over MCP. Claude Code, Cursor, Aider, Continue, your own scripts — they all get the same JSON-RPC surface to spawn shells, run commands, read pane state, capture screenshots, change settings, and record sessions.

The other 2026 terminals each pick a different side: Warp embeds AI inside a closed cloud (Oz), Ghostty stays out of your way and lets you bring your own tools, iTerm2 is Mac-only. Unterm picks the third side — terminal as MCP-controllable surface, deliberately keep AI features *out* of the terminal, let external agents grip it through the API.

Practical implications:

- Every Unterm window starts a local **MCP server** (line-delimited JSON-RPC over TCP) and a local **HTTP settings server** (Web Settings page) on auto-allocated ports. Both are auth-token gated, both are 127.0.0.1-only, no cloud round trip.
- **Settings live in the browser**, not the terminal. Cell-grid TUIs can't deliver modern form UX (no proper text inputs, no live preview, no color picker). The in-terminal `▼` overlay is intentionally minimal — six quick actions and a link to the Web Settings page.
- **9 languages out of the box**: en / 简体中文 / 繁體中文 / 日本語 / 한국어 / Deutsch / Français / Italiano / हिन्दी. Auto-detects from system locale, can be overridden in Web Settings or via `unterm-cli lang set <code>`.
- **Multi-instance discovery**: every running Unterm process owns one NATO-named instance (alpha, bravo, charlie…) and writes its ports + auth token to `~/.unterm/instances/<name>.json`. Agents that drive several windows at once enumerate that directory.
- **Cross-platform parity is a correctness property**: if a feature works on Windows but bails on macOS or Linux, that's a bug, not "not supported yet."
- **Subtraction over decoration**: no AI overlay inside the terminal, no inline image render that wedges the GUI, no in-terminal custom right-click chrome, no Cmd+Q confirmation, no manual proxy URL config (auto-detected from system). Finder integration on macOS uses the native Finder right-click extension and Services.

Built on top of the WezTerm engine for renderer / font / TUI / SSH / mux work, with a thin product layer on top.

---

## Install

Pre-built artifacts are published on GitHub Releases:

https://github.com/unzooai/unterm/releases

| Platform | Artifact                                                    |
| -------- | ----------------------------------------------------------- |
| macOS    | `Unterm-macos-<version>.dmg` (universal arm64+x86_64, signed + notarized) |
| Linux    | `unterm-<version>.deb` or `Unterm-<version>-x86_64.AppImage` |
| Windows  | `Unterm-<version>-x64.msi` or `Unterm-windows-<version>.zip` |

### macOS

Double-click `Unterm-macos-<version>.dmg`, then drag `Unterm.app` onto the
`Applications` shortcut. The DMG is signed with a Developer ID and Apple-
notarized, so Gatekeeper opens it on first launch without warnings.

Finder integration is bundled in the DMG. After the first launch, Finder's
right-click menu can show `Open in Unterm` for folders and files; if macOS
doesn't refresh the extension immediately, run `Repair Finder Integration.command`
from the DMG once.

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

## What's new

- **v0.12** — synchronous pre-show paint kills the long-standing Windows white flash on launch. The frame is rendered before the window is shown, not after.
- **v0.9** — multi-instance discovery with NATO-phonetic names. Each running Unterm writes `~/.unterm/instances/<name>.json`; `~/.unterm/active.json` points at the current foreground instance for single-target agents.
- **v0.7** — Windows defaults to UTF-8 out of the box. PowerShell and `cmd.exe` spawns are wrapped to set the code page; no more mojibake on Chinese filenames.
- **v0.5** — dogfood milestone. Default window sizing tuned for real use, scrollback line count is configurable via `~/.unterm/scrollback.json`, `TERM_PROGRAM` overridable via `~/.unterm/compat.json`, background update poller writes `~/.unterm/update_check.json`.

---

## Documentation

The full Unterm docs live at **https://unterm.app/docs/**:

- [Agent integration](https://unterm.app/docs/agent-integration) — how to drive Unterm from Claude Code / Cursor / Aider / your own client
- [MCP reference](https://unterm.app/docs/mcp-reference) — every JSON-RPC method, parameters, return shape
- [Multi-instance](https://unterm.app/docs/multi-instance) — NATO names, instances directory, picking the right window
- [CLI reference](https://unterm.app/docs/cli-reference) — `unterm-cli` subcommands, flags, exit codes
- [Configuration](https://unterm.app/docs/configuration) — every file under `~/.unterm/`
- [Architecture](https://unterm.app/docs/architecture) — what we forked from WezTerm and why

This README is the short version. The site is the long version.

---

## Features

- **GPU-accelerated rendering** on all three platforms (Metal / OpenGL / DirectX via ANGLE).
- **MCP server** on `127.0.0.1:<auto-port>` (default 19876) — JSON-RPC over TCP, auth-token gated. Method namespaces: session, exec, screen, signal, orchestrate, proxy, workspace, capture, policy, system, server, instance.
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

## Multi-instance

Every running Unterm process is one **instance** with a NATO-phonetic name: `alpha`, `bravo`, `charlie`, … `zulu`. The first window claims `alpha`, the second `bravo`, etc. When all 26 are taken at once, the next one wraps to `alpha2`. Names are easy to pronounce and AI agents handle them right — no UUIDs, no ports in your head.

Each instance writes its metadata (mcp_port, http_port, auth_token, pid, started_at, version, platform) to `~/.unterm/instances/<name>.json`. Agents that need to drive a specific window enumerate that directory and pick by id, cwd, or title. For single-target agents, `~/.unterm/active.json` points at the most recently launched live instance, and `~/.unterm/server.json` mirrors that same record for backward compat.

The MCP `instance.*` namespace exposes this directly: `instance.list`, `instance.info`, `instance.set_title`, `instance.focus`. See [the multi-instance docs](https://unterm.app/docs/multi-instance) for examples and the discovery protocol.

---

## CLI

The `unterm-cli` binary exposes the full Unterm product surface, transparently routing to the local MCP server. Read `~/.unterm/server.json` (or any file under `~/.unterm/instances/`) for current ports + auth.

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

Pass `--json` to any subcommand for raw JSON-RPC output (suitable for scripts). Pass `--lang <code>` to override the locale for one invocation.

Multi-instance discovery is exposed over MCP rather than as a CLI subcommand — call `instance.list` against any running Unterm's MCP port, or just `ls ~/.unterm/instances/`.

---

## Configuration

User config lives at:

| Platform | Location                                 |
| -------- | ---------------------------------------- |
| macOS    | `~/.unterm/`                             |
| Linux    | `~/.unterm/`                             |
| Windows  | `%USERPROFILE%\.unterm\`                 |

Files:

| File                         | Purpose                                          |
| ---------------------------- | ------------------------------------------------ |
| `server.json`                | Active instance's MCP/HTTP ports + auth token + pid (auto, mirrors the active instance for back-compat) |
| `active.json`                | Pointer at the current foreground instance id (auto, updated only when previous active dies) |
| `instances/<name>.json`      | Per-instance metadata (NATO id, ports, token, pid, started_at, version, platform) |
| `auth_token`                 | Legacy mirror of the active auth token (for back-compat) |
| `proxy.json`                 | `{"enabled": true|false}` — URLs auto-detected   |
| `theme.json`                 | Active theme id                                  |
| `lang.json`                  | Persisted locale override                        |
| `compat.json`                | `{"term_program": "..."}` override for `$TERM_PROGRAM` |
| `scrollback.json`            | Override the default scrollback line count       |
| `update_check.json`          | Background update-poller state (last check, latest seen version) |
| `onboarded.json`             | First-run flags (which `▼` items have been seen)  |
| `recording.json`             | Recording config (redaction patterns, etc.)      |
| `sessions/`                  | Recording metadata index (per-project subdirs)   |
| `screenshots/`               | Region screenshots (PNG)                         |

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
# Windows — MSI (requires WiX 6 at .\.tools\wix.exe — install via `dotnet tool install --tool-path .\.tools wix --version 6.0.1`)
pwsh -File ci/build-msi.ps1
```

macOS code-signing + notarization is **local-only** (no CI step) so the
Developer ID `.p12` private key never has to leave your Mac. One-time
setup, on the Mac that holds the cert:

```bash
xcrun notarytool store-credentials UntermNotary \
  --apple-id <your-apple-id> --team-id 6NQM3XP5RF
```

### Release tagging

Unterm uses **2-segment minor tags only**: `v0.7`, `v0.9`, `v0.12`. Patches accumulate as commits on `master` between releases — they don't get tagged and don't trigger CI builds. Cut a tag only when a coherent batch of fixes / features is ready to ship.

```bash
git tag -a vX.Y -m "Unterm vX.Y" && git push origin vX.Y
make release-mac                    # build universal + sign + notarize + upload
```

`make release-mac` reads the tag from `git describe --exact-match HEAD`,
builds universal x86_64+aarch64 binaries, calls `ci/sign-macos.sh` with
`NOTARY_PROFILE=UntermNotary`, then `gh release upload`s the resulting
zip to the matching GitHub Release.

CI on every PR runs `cargo check` against macOS, Linux, and Windows.
Tagged pushes (`vX.Y`) trigger the `release-linux` and `release-windows`
workflows that publish those two platforms' artifacts to GitHub Releases.
macOS sits out of CI by design — see above.

---

## Repository

This repository is the main Unterm project:

https://github.com/unzooai/unterm

Unterm includes modified WezTerm components. Upstream WezTerm remains a separate project by Wez Furlong and contributors.
