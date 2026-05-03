---
layout: ../../layouts/Doc.astro
title: Architecture overview
subtitle: How a keystroke becomes pixels — the layered model from MCP server through mux through pty through font shaper to GPU. For contributors and the curious.
kicker: Docs / Architecture
date: 2026-05-03
---

## One process, three control planes

Unterm runs as a single OS process per window. That process simultaneously hosts three control planes that all talk to the same in-memory `Mux`:

- The **GUI / GPU renderer** on the foreground thread — owns the OpenGL (or WebGPU) context, paints frames, dispatches OS input events.
- The **MCP server** on a worker thread — TCP listener bound to `127.0.0.1:19876` (preferred port; falls back to 19877+ if taken). Speaks line-delimited JSON-RPC 2.0, gates writes behind a UUID auth token.
- The **HTTP web settings server** on a worker thread — bound to `127.0.0.1:19877` (preferred), serves the Tailwind+Alpine SPA at `/` and a REST surface at `/api/*`.

All three are started from `async_run_terminal_gui` in `wezterm-gui/src/main.rs:416`. The MCP server is brought up first because it owns the UUID token; the HTTP server is then handed the same token so both servers share auth state. Both servers carry an `Arc<McpHandler>` and call directly into the `Mux` API — no IPC, no async runtime, no extra socket hop:

```text
                          ┌──────────────────────────────────────────┐
                          │            unterm (one process)          │
                          │                                          │
  user keypress ───►  ┌───┴────┐    ┌────────┐    ┌───────────────┐  │
                      │  GUI   │◄──►│        │◄──►│ local pty     │──┼─► child shell
                      │ thread │    │  Mux   │    │ reader thread │  │   (zsh, bash, …)
                      └───┬────┘    │        │    └───────────────┘  │
                          │         │        │                       │
  agent (Claude/CLI) ─┐   │         │ shared │                       │
  TCP 19876 ──────────┼──►│  MCP    │  state │                       │
                      │   │ thread  │        │                       │
                      │   └─────────┤        │                       │
  browser ────────────┐             │        │                       │
  TCP 19877 ──────────┼────────────►│  HTTP  │                       │
                      │             │ thread │                       │
                      │             └────────┘                       │
                      └──────────────────────────────────────────────┘
```

Both ports plus the auth token land in `~/.unterm/server.json` on launch. Per-instance metadata lands in `~/.unterm/instances/<nato>.json`. That file is the canonical handshake — every external tool, including `unterm-cli`, reads it to find the live instance.

This is the central architectural decision: the agent surface is _inside_ the same binary as the renderer, sharing memory with `Mux`. There is no "AI process" to crash separately, no message bus, no second source of truth for terminal state.

## Process model

```text
unterm                 (GUI process — one per window)
├── thread: main       (window event loop, GL, paint)
├── thread: mux-server (Unix socket listener for wezterm-client)
├── thread: mcp-server (TCP 19876)
├── thread: web-settings (TCP 19877)
├── thread: update-check (GitHub poller, 6h cycle)
└── thread: read-pty-N  (one per pane, blocking read on the master fd)
       │
       └── child process: $SHELL  (the user's zsh / bash / pwsh)
```

A few things worth pointing out:

- **`unterm-mux`** (binary built from `wezterm-mux-server/`) is a _separate_ binary for headless multiplexing — daemon mode for SSH-style "attach later from the GUI" workflows. The default desktop launch does **not** use it. The GUI process embeds its own `Mux` directly. The Unix socket at `$WEZTERM_RUNTIME_DIR/gui-sock-<pid>` is opened by a worker thread inside the GUI process (`wezterm-gui/src/main.rs:721`) so a second `unterm` invocation can talk to the first.
- **`unterm-cli`** (binary built from `wezterm/`) is short-lived. It opens a TCP socket to `127.0.0.1:19876`, calls `auth.login` with the token from `server.json`, runs one or more JSON-RPC methods, and exits. It never embeds a Mux of its own.
- **One window = one process**. Spawning a second window with `Cmd-N` from the menu actually re-execs `unterm` and the second process registers itself as `bravo` (or the next free NATO name) in `~/.unterm/instances/`. Tabs and panes are inside one window/process; new _windows_ are new processes.

## The crate stack

The workspace has ~40 crates inherited from WezTerm. The ones you'll touch most:

| Crate | Responsibility |
|---|---|
| `window/` | OS-level windows, GL/Metal/WebGPU contexts, raw key/mouse events. The `::window::Window` trait is what the GUI thread holds. |
| `wezterm-font/` | Font discovery (FreeType/CoreText/DirectWrite), glyph rasterization, HarfBuzz shaping. `FontConfiguration` lives here. |
| `termwiz/` | "Terminal Wizardry." The escape-sequence parser, cell model, surface diffing. `wezterm-escape-parser` is the low-level state machine. |
| `term/` | The virtual terminal emulator core — interprets `vtparse` actions into a 2D cell grid with attributes. Output of `read_from_pane_pty` flows through here. |
| `vtparse/` | Pure VT500-family escape parser. Stateless, table-driven, no allocations on the hot path. |
| `mux/` | The terminal multiplexer — owns all panes, tabs, windows, the per-pane PTY reader threads, and the notification fan-out. `Mux::get()` is the global. |
| `pty/` | Cross-platform PTY allocation (`openpty` on Unix, ConPTY on Windows). `portable_pty` is its public alias. |
| `wezterm-client/` | Client side of the mux protocol — used by `unterm` GUI to attach to a `unterm-mux` daemon over Unix socket / TLS. |
| `wezterm-mux-server-impl/` | Server side of the same mux protocol. Used both by the standalone `unterm-mux` daemon _and_ by the in-process socket worker the GUI starts. |
| `config/` | Loads `wezterm.lua`, exposes `ConfigHandle`. The Lua API surface for user config. |
| `wezterm-gui/` | The product layer. Everything Unterm-specific lives here: `mcp/`, `web_settings/`, `server_info.rs`, `system_proxy.rs`, `update_check.rs`, `i18n/`, `recording/`, `overlay/*` (settings menu, theme picker, command palette), `termwindow/` (the window controller). The binary built from this crate is `unterm`. |
| `wezterm/` | The CLI binary, built as `unterm-cli`. Hosts both the legacy `cli` subcommands (mux RPC) and the new `unterm_cli/` subcommands (MCP RPC). |

Everything below `wezterm-gui/` is a near-verbatim WezTerm crate; everything in `wezterm-gui/src/{mcp,web_settings,server_info,system_proxy,update_check,i18n,recording}` is Unterm-specific.

## The keystroke-to-pixel pipeline

Pick a single keypress — the user types `a`. Trace it:

```text
1. OS: NSWindow / X11 / Wayland delivers a key event to the window crate.
2. window:  Connection event handler converts to KeyEvent, fires window callback.
3. termwindow::keyevent::raw_key_event_impl   (wezterm-gui/src/termwindow/keyevent.rs:430)
4. inputmap::lookup_key   — does it match a configured key binding?
                           If yes: dispatch KeyAssignment (paste, spawn, …) and stop.
                           If no:  continue.
5. encode_kitty_input / encode_win32_input — optional protocol encoding for
                            apps that requested it via DECRPM (vim, helix, etc).
                            Falls back to raw bytes for plain mode.
6. pane.writer().write_all(b"a")       — writes to the pty master fd.
7. Kernel  copies bytes through pty into the slave fd.
8. Shell   reads from its stdin, processes (echoing in canonical mode), writes
            its rendering of the cursor advance back to the master fd.
9. read_from_pane_pty    (mux/src/lib.rs:299) — the per-pane blocking reader
                            thread reads bytes off the master fd into a 32K buffer.
10. parse_buffered_data  — feeds bytes to vtparse, which feeds Action events to
                            term::Terminal::perform_actions. Mutates the cell grid.
11. Mux fires MuxNotification::PaneOutput → invalidate the GUI window.
12. GUI thread next paint cycle calls termwindow::render::paint::paint_pass
                            (wezterm-gui/src/termwindow/render/paint.rs:162).
13. Per-line shaping: wezterm-font shapes runs of cells with HarfBuzz and
                            caches GlyphCache entries.
14. Quads are pushed to the GL layer. glium (or webgpu) submits the draw call.
15. SwapBuffers / Metal present.
```

The interesting hot paths:

- **Steps 6-8 cross the kernel.** This is where most "feels laggy" complaints come from. There's no work Unterm can do here.
- **Step 9 lives on a dedicated thread per pane.** It does blocking reads. If a pane spews multi-megabyte output (`yes`, `cat largefile`), this thread can saturate; `parse_buffered_data` is the buffer between it and the grid mutator.
- **Step 11's notification is coalesced.** The GUI doesn't repaint per-byte — it repaints once per frame (vsync) and absorbs all mutations that landed in the meantime.
- **Step 13 is the most allocation-heavy step.** `wezterm-font/src/shaper/` caches `GlyphInfo` aggressively; cache misses on cold lines (resize, theme switch) are the biggest single cost on a slow paint.

End-to-end target on a healthy machine is well under 16 ms (one vsync). Pathological cases — big terminal, complex CJK shaping, fresh font load — push into 30-60 ms. That's normal; below 16 ms is "feels native," above 33 ms is "feels laggy."

## The MCP control plane

The MCP server is a thin TCP-and-JSON wrapper around `McpHandler`, which is itself a thin wrapper around `Mux::get()`. Listener at `wezterm-gui/src/mcp/server.rs:51`; dispatch table at `wezterm-gui/src/mcp/handler.rs:101`.

The flow for a remote `session.input` (the agent types into a pane):

```text
agent → 127.0.0.1:19876
            │
            ▼
mcp-server thread (run_server)
            │  one thread::spawn per accepted connection
            ▼
mcp-client thread (handle_client)
            │  parses one JSON line; checks auth_token
            ▼
McpHandler::handle("session.input", params)
            │
            ▼
Mux::get_pane(id)            ← same global Mux the GUI is reading from
            │
            ▼
pane.writer().write_all(bytes)   ← pty master fd
```

That last step is identical to step 6 of the keystroke pipeline. From the shell's point of view, agent input and human input are byte-identical.

State that the MCP plane needs to mutate but the Mux doesn't naturally own — proxy settings, command policy, audit log, recording state — lives in a `OnceLock<Mutex<McpState>>` inside `handler.rs`. This is deliberately not in the Mux because it's product surface, not terminal-emulation surface; the Mux remains close to upstream.

When the MCP plane changes pane state (spawn, resize, kill), it dispatches the same `MuxNotification` events the GUI key path does. The GUI repaints, the agent never has to "tell the GUI to update." One Mux, one notification stream, three control planes reading and writing the same in-memory state.

## The Web Settings server

`wezterm-gui/src/web_settings/server.rs` is a hand-rolled HTTP/1.1 server — no async runtime, no Hyper, no Actix. Routes at `wezterm-gui/src/web_settings/server.rs:8`:

```text
GET  /                              — SPA shell (assets::INDEX_HTML)
GET  /static/<name>                 — bundled Tailwind / Alpine / app.js
GET  /bootstrap.json                — token + ports (no auth — so the SPA can self-bootstrap)
GET  /api/health                    — liveness
GET  /api/state                     — aggregate snapshot (theme, proxy, recording, instances)
POST /api/proxy                     — proxy_configure / proxy_disable
POST /api/theme                     — writes ~/.unterm/theme.json
POST /api/recording/start           — recording::start_recording
POST /api/recording/stop            — recording::stop_recording
GET  /api/sessions                  — list recorded sessions on disk
GET  /api/sessions/:id/markdown     — read a redacted session transcript
```

No keep-alive, no chunked encoding, no upgrade dance. Browsers handle that fine for a localhost-only SPA.

The SPA itself is `index.html` + `app.js` + `style.css` + vendored `tailwind.js` and `alpine.js`. All five files live in `wezterm-gui/assets/settings/` and are inlined into the binary at build time via `include_str!` (see `wezterm-gui/src/web_settings/assets.rs:8`). There is no JS build step. Editing the SPA is "edit the file, rebuild the binary." This is intentional: it keeps the contributor onboarding ramp on the SPA exactly the same as the Rust ramp.

When the user clicks "Apply theme" in the SPA:

```text
browser POST /api/theme {name: "dracula"}
            │
            ▼
HTTP handler thread
            │
            ▼
write ~/.unterm/theme.json     ← config-watcher in `config/` crate notices
            │
            ▼
ConfigHandle reload fires      ← all subscribers in the GUI get a notification
            │
            ▼
GUI repaints with new colors
```

The HTTP plane never touches GL state directly. It writes a config file; the existing config-reload pipeline does the rest.

## Discovery and multi-instance

`wezterm-gui/src/server_info.rs` is the discovery layer. Each launched `unterm` claims the lowest free NATO-phonetic name (`alpha`, `bravo`, `charlie` … `zulu`, then `alpha2`, `bravo2`, …) via O_EXCL atomic file creation. Conflicts when two instances start in the same millisecond resolve by retry on EEXIST.

On disk:

```text
~/.unterm/
├── instances/
│   ├── alpha.json    { id, mcp_port, http_port, auth_token, pid, started_at, … }
│   ├── bravo.json
│   └── charlie.json
├── server.json       — pointer to the active instance (back-compat for single-instance agents)
├── active.json       — explicit active-instance pointer
└── theme.json, lang.json, proxy.json, update_check.json, …
```

The MCP `instance.list` / `instance.info` / `instance.focus` / `instance.set_title` methods enumerate these. `unterm-cli session list` walks the directory and offers each instance to the user. `active.json` is updated only when the previous active instance dies — not on every focus event — so disk IO stays minimal.

For more on multi-instance discovery and the active-instance handoff, see the multi-instance doc (separate page).

## Why a fork, not a plugin

The obvious alternative would have been a WezTerm Lua plugin. Several Unterm features can't be implemented as Lua:

- **Local TCP servers.** Lua-on-WezTerm has no `TcpListener`. Spawning a JSON-RPC server from inside the Lua sandbox is not on offer.
- **HTTP server with bundled SPA.** Same constraint — and even if you could open the listener from Lua, embedding ~150 KB of Tailwind+Alpine assets at compile time wants `include_str!`, which is a Rust-side concern.
- **Recording with token redaction.** The redaction pipeline (`wezterm-gui/src/recording/redact.rs`) wants to scan raw byte streams as they flow through `parse_buffered_data` — Lua-side hooks fire after escape parsing, too late for byte-level masking.
- **macOS code signing + notarization + the WiX MSI.** `ci/sign-macos.sh` and `installer/Unterm.wxs` need a binary they own — `wezterm` codesigned binaries with the user's plugin loaded would require either a separate signed binary anyway (which is the fork) or running unsigned (which is a non-starter for distribution).
- **i18n.** The translated string set spans menus, modals, settings UI, CLI output, and toast notifications. Threading nine locale dictionaries through `wezterm.lua` would be its own product.
- **Multi-instance discovery and per-instance NATO naming.** Has to be in the binary so that the GUI process knows its own identity from the moment it boots.

So the fork is honest about what it is: a thin product layer on top of WezTerm's renderer, font, mux, and SSH stack, owning the build pipeline, the binary identity, and the user-facing surface. Upstream WezTerm is left untouched in our tree (everything outside `wezterm-gui/src/{mcp,web_settings,server_info,system_proxy,update_check,i18n,recording}` and the renamed binaries is essentially upstream).

## What changes upstream from WezTerm

The Unterm-specific surface, top to bottom:

| Path | Purpose |
|---|---|
| `wezterm-gui/src/mcp/` | MCP JSON-RPC server: server.rs (TCP listener), handler.rs (~50 methods). |
| `wezterm-gui/src/web_settings/` | Hand-rolled HTTP/1.1 server + bundled SPA loader. |
| `wezterm-gui/src/server_info.rs` | Multi-instance NATO naming + on-disk discovery. |
| `wezterm-gui/src/system_proxy.rs` | OS proxy auto-detection (scutil / gsettings / registry / port scan). |
| `wezterm-gui/src/update_check.rs` | Background GitHub release poller. |
| `wezterm-gui/src/i18n/` | 9-locale translation table with embedded JSON dictionaries. |
| `wezterm-gui/src/recording/` | Pane-byte recording with token/secret redaction → markdown. |
| `wezterm-gui/src/session_state.rs` | Last-window position/size persistence (the menu bar restore). |
| `wezterm-gui/src/overlay/{settings_menu,theme_selector,proxy_settings,shell_selector,tab_context_menu}.rs` | Unterm-specific overlay screens. |
| `wezterm-gui/assets/settings/` | The web settings SPA — index.html, app.js, style.css, vendored Tailwind+Alpine. |
| `wezterm/src/unterm_cli/` | CLI subcommands that dispatch over MCP rather than the legacy mux protocol. |
| `ci/sign-macos.sh`, `ci/release-mac.sh` | Codesign + notarize pipeline. |
| `installer/Unterm.wxs` | Windows MSI definition. |
| `assets/macos/`, `assets/icon/` | Renamed bundle ID (`com.unzoo.unterm`), Unterm.icns. |
| Binary names | `unterm` (GUI), `unterm-cli`, `unterm-mux`. The crate names were not renamed (`wezterm-gui` still builds the `unterm` binary) so upstream merges stay tractable. |

## Where to start contributing

By intent:

**Adding a new MCP method** — `wezterm-gui/src/mcp/handler.rs`. Add a match arm in `McpHandler::handle` (~line 101), implement the handler. If the method needs new persistent state, it goes either in `~/.unterm/<feature>.json` (write through `serde_json::to_writer`) or in the `McpState` struct if it's per-process and ephemeral. Add the method to `server_capabilities` (~line 280) so introspection works.

**Adding a Web Settings page** — three places: a route in `wezterm-gui/src/web_settings/server.rs:8`, a section in `wezterm-gui/assets/settings/index.html`, an Alpine component in `wezterm-gui/assets/settings/app.js`. The data plumbing should call existing `McpHandler` methods rather than reimplementing the operation — the SPA is a UI for the MCP surface, not a parallel implementation.

**Adding a new CLI subcommand** — `wezterm/src/unterm_cli/<name>.rs`, registered in `wezterm/src/unterm_cli/mod.rs:8`. The new command should be a thin client over the MCP method you already added — see `proxy.rs` for the pattern.

**Fixing a render bug** — `wezterm-gui/src/termwindow/render/`. The frame entry point is `paint_pass` at `paint.rs:162`. Per-line shaping lives in `wezterm-font/src/shaper/`. If the bug is in the cell grid (wrong character, wrong attribute), it's upstream of paint — look at `term/` and `termwiz/`.

**Fixing a font / shaping bug** — `wezterm-font/`. Shapers under `shaper/`, font discovery under `locator/`, rasterizers under `rasterizer/`. The HarfBuzz wrapper is `hbwrap.rs`, FreeType is `ftwrap.rs`. CoreText is the macOS preferred path; FreeType+HarfBuzz is the cross-platform fallback.

**Changing the multi-instance discovery semantics** — `wezterm-gui/src/server_info.rs`. The on-disk format is documented at the top of that file. Be careful with `active.json` semantics: the design lock-in (2026-05-02) is that it updates only on instance death, not on focus, so the disk write rate stays bounded.

**Touching the MCP auth model** — `wezterm-gui/src/mcp/server.rs:114`. The auth-login dance happens once per connection. The token is regenerated on every launch and lives in `~/.unterm/server.json` with mode 0600. Don't add a way to read it from the LAN.

**Adding a translation** — `wezterm-gui/src/i18n/locales/`. Copy `en.json`, translate values, register the new code in `LOCALE_CODES` and `LOCALE_BUNDLES` in `wezterm-gui/src/i18n/mod.rs:29`. The CLI, GUI menus, and Web Settings dictionaries all read from the same embedded data.

**Touching session recording** — `wezterm-gui/src/recording/`. `recorder.rs` is the lifecycle, `redact.rs` is the token-masking pass, `render.rs` is the markdown emission, `index.rs` is the on-disk session listing. Recordings are byte streams, not key streams — the redactor sees what the shell actually wrote, not what the user typed.

---

If you're filing a PR and you're not sure where it fits, the rule of thumb: anything that an external agent should be able to do, the MCP handler is the right entry point, and everything else (CLI, Web Settings, GUI menu) should layer on top of that handler rather than reimplement it. The MCP handler is the single product API; the rest are presentations.
