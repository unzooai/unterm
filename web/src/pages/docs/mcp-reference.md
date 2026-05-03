---
layout: ../../layouts/Doc.astro
title: MCP Method Reference
subtitle: Every method exposed by the local Unterm MCP server, with parameter shapes, return shapes, error codes, and a real example.
kicker: Docs / MCP reference
date: 2026-05-03
---

This page documents every JSON-RPC method an MCP client can call against a running Unterm instance. The dispatch table lives in `wezterm-gui/src/mcp/handler.rs`; the connection handshake is in `wezterm-gui/src/mcp/server.rs`. Both are MIT-licensed in the public repo.

For higher-level patterns (director/worker, multi-pane orchestration, recording for review) see the [agent integration guide](agent-integration). This page is the wire-level companion — the doc you check when your client got back `-32603` and you want to know which field you fat-fingered.

## Connection and auth

### Where the port and token live

On launch, every Unterm process writes its identity to three files under `~/.unterm/`:

- `~/.unterm/instances/<nato-name>.json` — the canonical record for *this* instance. NATO-phonetic ids: `alpha`, `bravo`, `charlie`, … cycling to `alpha2` when all 26 are taken simultaneously. Contains `mcp_port`, `http_port`, `auth_token`, `pid`, `started_at`, `title`, `cwd`, `version`, `platform`.
- `~/.unterm/server.json` — single-instance compat alias. Mirrors the *active* instance's metadata. Older agents that only know about one Unterm at a time read this and keep working.
- `~/.unterm/active.json` — pointer to the currently active instance id. Updated only when the previous active dies, not on every focus change. Disk-IO budget.

A multi-instance-aware agent should enumerate `~/.unterm/instances/*.json`, drop entries whose `pid` is no longer live, and pick the instance it wants by `title`, `cwd`, `started_at`, or whatever heuristic it prefers. A single-instance-aware agent just reads `server.json` and ignores the rest.

The MCP server preferred port is `19876` (HTTP settings server is `19877`). On collision, Unterm walks forward up to `PORT_RETRY_LIMIT` (5) ports before giving up, so in practice you'll see ports in `19876..=19881`. Both bind to `127.0.0.1` only — nothing on the LAN can reach them.

### Framing

The protocol is line-delimited JSON-RPC 2.0 over TCP:

- Each request is one line of JSON, terminated by `\n`.
- Each response is one line of JSON, terminated by `\n`.
- TCP `nodelay` is set on the server side, so small frames flush immediately.
- Empty lines are skipped.
- Parse errors return `{ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": "Parse error: ..." } }` and the connection stays open.

There is no batch support, no notifications (every request gets a response), no `params` schema validation beyond what the handler does itself.

### The auth handshake

The very first method on a new TCP connection MUST be `auth.login`, with the token from `instances/<id>.json`:

```json
{"jsonrpc":"2.0","id":1,"method":"auth.login","params":{"token":"5f3c2a1e-..."}}
```

Success returns:

```json
{"jsonrpc":"2.0","id":1,"result":{"status":"ok"}}
```

Wrong token returns error code `-32001` (`"Invalid auth token"`) and the connection is *not* dropped — you can retry with the right token. Calling any other method before `auth.login` returns `-32002` (`"Not authenticated. Call auth.login first"`). Once authenticated, the connection stays authenticated for its lifetime.

### Error codes

| Code | When |
|---|---|
| `-32700` | Parse error — request line wasn't valid JSON |
| `-32001` | Invalid auth token (bad credentials on `auth.login`) |
| `-32002` | Not authenticated (any method before `auth.login`) |
| `-32603` | Internal error — handler returned `Err`. The `message` field is the underlying anyhow error, e.g. `"Session 7 not found"` or `"Missing 'command'"` |

There is no `-32601` (method not found); unknown methods come back as `-32603` with message `"Unknown method: <name>"`.

### A complete handshake-and-call

```
> {"jsonrpc":"2.0","id":1,"method":"auth.login","params":{"token":"5f3c..."}}
< {"jsonrpc":"2.0","id":1,"result":{"status":"ok"}}
> {"jsonrpc":"2.0","id":2,"method":"session.list","params":{}}
< {"jsonrpc":"2.0","id":2,"result":{"sessions":[{"id":0,"title":"zsh","cols":120,"rows":30,...}]}}
```

That's the entire protocol. Everything below is just which methods you can put in the `method` field and what each one does.

---

## Session

The session namespace is the primary surface — every pane in the terminal is a "session" with a numeric id. Most other namespaces (`exec`, `screen`, `capture`, recording) take a session id as their first parameter.

A note on parameter naming: pane id can be passed as either `id` (numeric) or `session_id` (string). Both work everywhere a pane is required. The CLI tends to use `id`; older clients use `session_id`. They're aliases.

### `session.list`

Enumerate every live pane. No params.

**Returns:** `{ sessions: [{ id, title, cols, rows, cursor: { x, y, visible }, is_dead, domain_id, shell: { shell_type, process_name, cwd } }] }`

`shell_type` is one of `"powershell"`, `"cmd"`, `"bash"`, `"zsh"`, `"fish"`, `"nushell"`, `"unknown"` — derived by parsing the foreground process name.

```json
{"jsonrpc":"2.0","id":3,"method":"session.list","params":{}}
```

```json
{"jsonrpc":"2.0","id":3,"result":{"sessions":[
  {"id":0,"title":"alex@laptop ~/code/unterm","cols":120,"rows":30,
   "cursor":{"x":2,"y":29,"visible":true},
   "is_dead":false,"domain_id":0,
   "shell":{"shell_type":"zsh","process_name":"/bin/zsh","cwd":"file:///Volumes/Dev/code/unterm"}}
]}}
```

### `session.get` / `session.status`

Same method, two names. Get full state for one pane, including scrollback row count.

**Params:** `id` (number) or `session_id` (string), required.

**Returns:** `{ id, title, cols, rows, scrollback_rows, cursor: { x, y, visible }, is_dead, domain_id, shell }`

### `session.create`

Spawn a new pane in the active window using the default domain.

**Params:**

| Name | Type | Required | Description |
|---|---|---|---|
| `cols` | number | no | Terminal width, default `120` |
| `rows` | number | no | Terminal height, default `30` |
| `cwd` | string | no | Initial working directory; defaults to user home |

**Returns:** `{ id, session_id, title, cols, rows }`

The call blocks for up to 10 seconds waiting for the pty to come up. It runs the user's default shell — there is currently no `prog` parameter to launch a non-shell process directly. If you need that, `session.create` then `exec.run` with the command.

```json
{"jsonrpc":"2.0","id":4,"method":"session.create",
 "params":{"cwd":"/Volumes/Dev/code/unterm","cols":160,"rows":48}}
```

```json
{"jsonrpc":"2.0","id":4,"result":{"id":7,"session_id":"7","title":"zsh","cols":160,"rows":48}}
```

### `session.input` / `exec.send`

Aliases. Write arbitrary bytes into the pane's stdin, exactly as if the user had typed them. Does *not* append a newline.

**Params:**

| Name | Type | Required | Description |
|---|---|---|---|
| `id` / `session_id` | number/string | yes | Target pane |
| `input` | string | yes | Raw characters to write |

**Returns:** `{ status: "ok" }`

If you want to submit a command, you almost always want `\r` (carriage return) at the end. Most shells treat `\n` as a literal line continuation; `\r` is what a real keypress sends.

### `session.resize`

Resize the pane's pty. Does what SIGWINCH would do — the running program receives the resize and reflows.

**Params:** `id`/`session_id` (yes), `cols` (yes), `rows` (yes).

**Returns:** `{ status: "ok" }`

### `session.destroy`

Kill the pane. Sends a kill to the underlying process and audits the action.

**Params:** `id`/`session_id`.

**Returns:** `{ status: "ok", destroyed: true }`

### `session.idle`

Heuristic check: is the foreground process the shell itself (idle) or a child (running)?

**Params:** `id`/`session_id`.

**Returns:** `{ idle: bool, foreground_process: string }`

`idle` is `true` when the foreground process name contains one of `powershell`, `pwsh`, `cmd`, `bash`, `zsh`, `fish`, `nu`. Anything else returns `false`. This is the call to use when polling "did my long-running build finish?"

### `session.cwd`

Get the pane's current working directory (from OSC 7 if the shell sets it, falls back to inspection).

**Params:** `id`/`session_id`.

**Returns:** `{ cwd: string }` — a `file://` URI string. May be empty if the shell doesn't emit OSC 7 and inspection failed.

### `session.env` / `session.set_env`

Read or write a pane's environment variables. **Currently stubs** — both return `{ value: null, message: "Environment variable reading not supported in WezTerm mode" }` (or `set`/`status: ok` equivalent). Don't rely on these. If you need to set env for a child process, prepend `export FOO=bar; ` to the command via `exec.run` instead.

### `session.history`

Return the last N lines of scrollback as a "history" list, with empty lines filtered out. This is *not* shell history (`~/.zsh_history`); it's pane scrollback.

**Params:**

| Name | Type | Required | Description |
|---|---|---|---|
| `id`/`session_id` | number/string | yes | Target pane |
| `limit` | number | no | Number of trailing rows to read, default `100` |

**Returns:** `{ entries: [{ text: string }, ...], count: number }`

### `session.audit_log`

Read the in-memory audit log. Every mutating method (`session.destroy`, `exec.run`, `signal.send`, `policy.set`, recording start/stop) appends an entry; reads do not. The log is process-local — restarting Unterm clears it.

**Params:**

| Name | Type | Required | Description |
|---|---|---|---|
| `limit` | number | no | Max entries to return, newest first, default `50` |
| `session_id` | string | no | Filter to a single pane id |

**Returns:** array of `{ timestamp (RFC3339), method, session_id, detail, allowed }`.

### Recording: `session.recording_start`, `_stop`, `_status`, `_list`, `_read`, `_attach_trace`, `session.export_markdown`

These wrap the `crate::recording` module. They write an OSC-133-aware redacted markdown transcript to disk (under `<cwd>/.unterm/sessions/<date>/` if there's a writable project directory, else `~/.unterm/sessions/_orphan/`). Tokens, GitHub PATs, and 40+ char base64/hex strings are masked before the file is written.

**`session.recording_start`** — begin recording one pane.

- Params: `id`/`session_id`.
- Returns: `{ session_id, log_path, md_path_when_done }` — `log_path` is the raw byte log being written live; `md_path_when_done` is where the final markdown will land when you call `_stop`.

**`session.recording_stop`** — finish, render the markdown, return paths and counts.

- Params: `id`/`session_id`.
- Returns: `{ session_id, ended_at, block_count, exit_reason, md_path }`. `block_count` is how many OSC 133 prompt boundaries were captured.

**`session.recording_status`** — non-mutating: is this pane currently being recorded?

- Params: `id`/`session_id`.
- Returns: whatever `crate::recording::recording_status` produces — typically `{ recording: bool, session_id, started_at, block_count }` (or just `{ recording: false }`).

**`session.recording_list`** — enumerate completed recordings on disk.

- Params: optional `project` (string) to filter by project path.
- Returns: array of `{ unterm_session_id, tab_id, project_path, project_slug, started_at, ended_at, block_count, bytes_raw, log_path, md_path }`.

**`session.recording_read`** — slurp one recording's rendered markdown back into memory.

- Params: `session_id` (string, required) — the recording's `unterm_session_id`, *not* a pane id.
- Returns: `{ markdown: string }`.

**`session.recording_attach_trace`** — associate an external trace id (e.g. an outer agent's correlation id) with a live recording. Useful when you want to correlate the markdown back to the agent's own logs after the fact.

- Params: `id`/`session_id` (pane), `trace_id` (string, required).
- Returns: `{ trace_ids: [...] }` — full list of trace ids attached so far.

**`session.export_markdown`** — render a one-off markdown of a pane's *current* scrollback, no recording session needed. The output is the same redacted format used by recordings, just without the streaming hook.

- Params: `id`/`session_id`, optional `path` (string) — destination file. If omitted, the recording module picks a default under the project's `.unterm/sessions/`.
- Returns: `{ session_id, path, bytes, block_count }`. The `session_id` here is a freshly-generated UUID, not a recording id.

---

## Exec

Higher-level wrappers around `session.input` for the common case of "run a command". Most agents will reach for these instead of typing `\r` themselves.

Every exec method is policy-checked: if `policy.set` has been called with `enabled: true` and the command matches a `blocked_patterns` substring, the call returns `-32603` with message `"Command blocked by policy: <pattern>"`. See the [Policy](#policy) section.

### `exec.run`

Send a command and a carriage return. Returns immediately — does not wait for the command to finish.

**Params:**

| Name | Type | Required | Description |
|---|---|---|---|
| `id`/`session_id` | number/string | yes | Target pane |
| `command` | string | yes | Shell command to run |

**Returns:** `{ sent: true }`

```json
{"jsonrpc":"2.0","id":5,"method":"exec.run",
 "params":{"id":7,"command":"cargo test --workspace"}}
```

### `exec.run_wait`

Send a command, *append a shell-specific sentinel*, and poll the pane's text every 200ms until the sentinel appears. Returns the captured output diff.

**Params:**

| Name | Type | Required | Description |
|---|---|---|---|
| `id`/`session_id` | number/string | yes | Target pane |
| `command` | string | yes | Shell command to run |
| `timeout_ms` | number | no | How long to wait, default `30000` |

**Returns:** `{ output: string, exit_status: "completed" | "timeout", timed_out: bool, marker: string }`

The sentinel is a fresh UUID-based string (`__UNTERM_DONE_<uuid>__`) appended after the command via shell-appropriate syntax: `; echo …` for unix shells, `; Write-Output …` for PowerShell, `& echo …` for `cmd`. The `output` field is the diff between pre- and post-execution screen text, with the command line and the sentinel stripped.

This is the "blocking subprocess" pattern: simple, but heuristic — it can confuse multi-line prompts, programs that redraw the screen (htop, vim), or commands that themselves contain the sentinel as a literal. For those, prefer `exec.run` + manual polling with `screen.search`.

### `exec.status`

Probe whether the foreground process looks like a shell or like a running command.

**Params:** `id`/`session_id`.

**Returns:** `{ status: "idle" | "running", foreground_process: string }`

Same heuristic as `session.idle` but with a different return shape. Either is fine; pick whichever your client code is already using.

### `exec.cancel`

Send Ctrl+C (`\x03`) to the pane.

**Params:** `id`/`session_id`.

**Returns:** `{ cancelled: true }`

### `exec.send`

Alias for `session.input`. Documented under [Session](#session).

---

## Signal

### `signal.send`

Send a control signal as a control character to the pane. Cross-platform — the actual POSIX signal isn't sent; the appropriate Ctrl-character is.

**Params:**

| Name | Type | Required | Description |
|---|---|---|---|
| `id`/`session_id` | number/string | yes | Target pane |
| `signal` | string | yes | One of `SIGINT`/`INT`, `SIGTSTP`/`TSTP`, `SIGQUIT`/`QUIT`, `EOF` |

**Returns:** `{ sent: true, signal: string }` on success, or `-32603` `"Unsupported signal: ..."` for anything else.

The bytes sent: `SIGINT`→`\x03`, `SIGTSTP`→`\x1a`, `SIGQUIT`→`\x1c`, `EOF`→`\x04`. On Windows the same bytes go in; the shell decides what to do with them.

```json
{"jsonrpc":"2.0","id":6,"method":"signal.send","params":{"id":7,"signal":"SIGINT"}}
```

---

## Screen

Read what's on the pane right now. None of these methods mutate state.

### `screen.read`

Visible viewport with absolute row indices and per-cell info.

**Params:** `id`/`session_id`.

**Returns:** `{ cells: [{ row, text }, ...], cursor: { x, y, visible }, cols, rows, scrollback_rows }`

Each cell entry covers one row, not one cell — the name reflects an older intention. `text` is the row trimmed of trailing whitespace.

### `screen.text`

Same as `screen.read` but the rows come back as a flat `lines: string[]` instead of `cells: [{row,text}]`. Use this when you don't care about absolute row numbers (you usually don't).

**Params:** `id`/`session_id`.

**Returns:** `{ lines: string[], cursor: { x, y }, cols, rows }`

### `screen.cursor`

Cursor position and shape only.

**Params:** `id`/`session_id`.

**Returns:** `{ x, y, visible, shape }` — `shape` is the `Debug` formatting of the underlying `CursorShape` enum, e.g. `"Default"`, `"BlinkingBlock"`, `"SteadyUnderline"`.

### `screen.scroll`

Read an absolute slice of the scrollback. Use this when you want history before what's visible.

**Params:**

| Name | Type | Required | Description |
|---|---|---|---|
| `id`/`session_id` | number/string | yes | Target pane |
| `offset` | number | no | Starting row, default `0` |
| `count` | number | no | Number of rows to read, default `100` |

**Returns:** `{ lines: string[], offset: number, count: number }` (count = lines actually returned).

### `screen.search`

Substring search across visible viewport + scrollback.

**Params:**

| Name | Type | Required | Description |
|---|---|---|---|
| `id`/`session_id` | number/string | yes | Target pane |
| `pattern` | string | yes | Literal substring (not regex) |
| `max_results` | number | no | Cap on matches, default `50` |

**Returns:** `{ matches: [{ row, text }, ...], total: number }`

Match is `String::contains`, case-sensitive, no regex. If you need regex, do it client-side after fetching `screen.text`.

### `screen.detect_errors`

Run a hardcoded error-pattern scan over the visible viewport.

**Params:** `id`/`session_id`.

**Returns:** `{ has_errors: bool, errors: [{ row, text, pattern }] }`

The pattern list is fixed in the binary: `error:`, `Error:`, `ERROR:`, `error[`, `fatal:`, `Fatal:`, `FATAL:`, `panic:`, `PANIC:`, `not found`, `command not found`, `Permission denied`, `permission denied`, `No such file or directory`, `failed`, `FAILED`, `traceback`, `Traceback`, `Exception`, `exception:`, `segfault`, `Segmentation fault`. First match per row wins.

This is meant for "does this look like the build broke?" not for serious log analysis.

---

## Capture

Screen and clipboard captures. PNG output goes to `~/.unterm/screenshots/`; clipboard images go to `~/.unterm/clipboard/`. Both directories are created on demand.

### `capture.screen`

Snapshot every pane's text plus a full-display PNG.

**Params:**

| Name | Type | Required | Description |
|---|---|---|---|
| `include_base64` | bool | no | Inline the PNG bytes as base64 in the response, default `false` |

**Returns:** `{ captures: [{ session_id, title, screen, type: "text" }, ...], image: { path, ... }, type: "image/png", text_snapshot: true }`

`image.path` is the absolute path to the PNG on disk. With `include_base64: true` the response also gets `image.base64`.

### `capture.window`

Snapshot one specific window — by partial title match or by pid. Returns one pane's text + the windowed PNG.

**Params:**

| Name | Type | Required | Description |
|---|---|---|---|
| `title` | string | no | Substring matched against pane titles and pane ids |
| `pid` | number | no | Process id of the window to capture |
| `include_base64` | bool | no | Inline base64 PNG, default `false` |

**Returns:** `{ session_id?, title?, screen?, image: {...}, type: "image/png", text_snapshot: bool }`

If neither `title` nor `pid` matches a known pane, only the image and `text_snapshot: false` come back — the OS-level windowed capture still runs.

### `capture.select`

Used to mean "interactive region selection". In headless MCP mode this is impossible — there's no GUI to draw the selection rectangle on — so the call falls back to a full-screen capture and notes that in the response.

**Params:** none.

**Returns:** `{ image: {...}, type: "image/png", mode: "screen_fallback", message: "Interactive region selection is not available in headless MCP mode; captured the screen instead." }`

### `capture.clipboard`

Read the OS clipboard. Cross-platform: Win32 `OpenClipboard`/`GetClipboardData` on Windows, `pbpaste` on macOS, `xclip`/`wl-paste` on Linux.

**Params:** none.

**Returns:** depends on what's on the clipboard.

- Text: `{ type: "text", content: "..." }`
- Image: `{ type: "image", format: "png", image_path, width, height, bit_depth, size_bytes, base64 }` — the image is always saved to `~/.unterm/clipboard/clipboard_<timestamp>.png` and base64 is always included for images (in contrast to `capture.screen`/`capture.window` where base64 is opt-in).

Errors if the clipboard is empty or contains an unsupported format.

---

## Proxy

Read and write the proxy configuration that lives at `~/.unterm/proxy.json`. The values flow through to environment variables (`HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, `NO_PROXY`) for child processes spawned by Unterm. Auto-detection runs from `system_proxy::detect()` when the user is in `mode: "auto"`.

### `proxy.status`

Current proxy state plus a live reachability probe when enabled.

**Params:** none.

**Returns:** `{ enabled, mode, http_proxy, socks_proxy, no_proxy, current_node, node_count, health }`

`health` is `null` when disabled, otherwise `{ source, url, reachable, hint? }`. `source` is `"manual"` (user set explicit URL), or whatever `system_proxy::detect` says (typically `"system"`, `"clash"`, or `"auto"`). `hint` is a human-readable message when `reachable: false`.

### `proxy.nodes`

List configured proxy nodes (named upstream URLs) and which one is current.

**Params:** none.

**Returns:** `{ current_node: string|null, nodes: [{ name, url, latency_ms, available }, ...] }`

Latencies and availability are populated by `proxy.speedtest`; reading `proxy.nodes` doesn't probe anything fresh.

### `proxy.switch`

Activate one of the configured nodes by name. Sets `enabled: true`, `mode: "manual"`, and writes through to `proxy.json`.

**Params:** `node_name` (string, required).

**Returns:** `{ switched: true, current_node, http_proxy }`

Errors with `"Proxy node '<name>' not found"` if the name doesn't match.

### `proxy.speedtest`

Probe one node (or all of them) and write `latency_ms` + `available` back to disk.

**Params:**

| Name | Type | Required | Description |
|---|---|---|---|
| `node_name` | string | no | Limit to one node; omit to probe all |
| `timeout_ms` | number | no | Per-node TCP connect timeout, default `3000` |

**Returns:** `{ results: [{ name, url, available, latency_ms }, ...] }`

The probe is a `TcpStream::connect_timeout` to the host:port parsed out of the URL. SOCKS, HTTP, and HTTPS URLs all work — no actual proxy protocol is exercised, just the TCP layer.

### `proxy.configure`

Write a full proxy config in one call: enabled flag, mode, manual URLs, full node list, and current node. This is the one to call when setting up proxy from scratch.

**Params:**

| Name | Type | Required | Description |
|---|---|---|---|
| `enabled` | bool | no | Default `true` |
| `mode` | string | no | `"manual"` or `"auto"`, default `"manual"` (ignored if `enabled: false`, gets forced to `"off"`) |
| `http_proxy` | string | no | URL like `"http://127.0.0.1:7890"` |
| `socks_proxy` | string | no | URL like `"socks5://127.0.0.1:7891"` |
| `no_proxy` | string | no | Comma-separated bypass list |
| `nodes` | array | no | `[{ name, url|http_proxy }, ...]` — replaces existing node list |
| `current_node` | string | no | Picks a node by name; sets its url as `http_proxy` |

**Returns:** `{ configured: true, status: <result of proxy.status> }`

### `proxy.disable`

Turn the proxy off. Equivalent to `configure` with `enabled: false`.

**Params:** none.

**Returns:** `{ disabled: true }`

### `proxy.env`

Resolve the proxy state to environment-variable form, doing the same auto-detection logic that Unterm uses when spawning child processes.

**Params:** none.

**Returns:** `{ enabled, env: { HTTP_PROXY?, HTTPS_PROXY?, ALL_PROXY?, NO_PROXY? } }`

When disabled, `env` is empty. When enabled, manual URLs win over auto-detected ones; if both are missing, only `NO_PROXY` ends up populated.

---

## Workspace

Save and restore named layouts of pane (cwd, title) tuples to `~/.unterm/workspaces/<name>.json`. The current implementation is intentionally minimal — restore returns the saved data but does *not* recreate the panes for you yet.

### `workspace.save`

Snapshot the current set of panes.

**Params:** `name` (string, required).

**Returns:** `{ saved: true, name, sessions: number }`

### `workspace.restore`

Read back a saved workspace.

**Params:** `name` (string, required).

**Returns:** `{ restored: true, name, workspace: { name, sessions: [{id, title, cwd}], saved_at }, message: "Workspace data loaded. Use session.create with cwd to recreate sessions." }`

The honest "to recreate, call `session.create` yourself for each saved session" message is the API right now. A full auto-restore is on the roadmap.

### `workspace.list`

Enumerate available workspace names.

**Params:** none.

**Returns:** `{ workspaces: [{ name }, ...] }`

---

## Orchestrate

Multi-pane convenience methods. These are thin wrappers over `session.create` + `session.input` — you can build the same patterns by hand if you prefer.

### `orchestrate.launch`

`session.create` plus a 500ms wait plus an `exec.run`-style command send. Used when you want to open a pane and immediately run something in it.

**Params:** same as `session.create` (`cwd`, `cols`, `rows`) plus `command` (string, optional).

**Returns:** same as `session.create` — `{ id, session_id, title, cols, rows }`.

If `command` is omitted, this is identical to `session.create`. If supplied, the command is sent with a carriage return after a 500ms shell-init delay.

### `orchestrate.broadcast`

Send the same command to multiple panes in one call.

**Params:**

| Name | Type | Required | Description |
|---|---|---|---|
| `command` | string | yes | Command to send (followed by `\r` per pane) |
| `sessions` | array of strings | yes | Pane ids as decimal strings |

**Returns:** `{ results: [{ session_id, sent?, error? }, ...] }`

Bad ids and missing panes don't fail the whole call — they show up as `error` entries in `results`.

### `orchestrate.wait`

Poll one pane until its text contains a substring, or the timeout fires.

**Params:**

| Name | Type | Required | Description |
|---|---|---|---|
| `id`/`session_id` | number/string | yes | Target pane |
| `pattern` | string | yes | Literal substring (no regex) |
| `timeout_ms` | number | no | Default `10000` |

**Returns:** `{ matched: true, pattern }` on hit, `{ matched: false, timed_out: true, pattern }` on timeout. Polls every 200ms.

This is the "wait for the inner agent's prompt marker before sending the next instruction" primitive. Compare with `exec.run_wait` which does its own sentinel injection.

---

## Theme

There is no `theme.*` namespace on the MCP server. Theme switching is done through the HTTP settings server at `127.0.0.1:<http_port>` — the same `~/.unterm/server.json` file lists that port. The HTTP server exposes the Tailwind+Alpine settings SPA at `/` and REST endpoints under `/api/settings/...`.

If you see references to `theme.list` / `theme.switch` in older docs, those are HTTP endpoints, not MCP methods. The MCP wire protocol does not currently surface theme management.

---

## Instance

Multi-instance discovery. Each Unterm process is one "instance" with a NATO-phonetic id, and each instance owns its own MCP port + auth token. To drive a peer instance, you connect to *its* MCP port directly with *its* token — there's no cross-instance forwarding through your local connection.

### `instance.list`

Enumerate every live Unterm instance on this machine. Stale entries (PID dead) are filtered out by the storage layer.

**Params:** none.

**Returns:** `{ instances: [{ id, pid, started_at, mcp_port, http_port, title, cwd, version, platform }, ...] }`

Note that this *omits* `auth_token` — the listing tells you a peer exists and where to find it, but to actually talk to a peer you read the peer's `~/.unterm/instances/<id>.json` file directly to grab its token.

### `instance.info`

This instance's own metadata, *including* its auth token. Useful for confirming "yes, I'm talking to the right window".

**Params:** none.

**Returns:** `{ id, pid, started_at, mcp_port, http_port, auth_token, title, cwd, version, platform }`

### `instance.set_title`

Pin a custom display title for this instance. Overrides the auto-derived `Unterm — <id> — <project>` window title and shows up in `instance.list` so peers can route to the right window. Pass `null` (or omit `title`) to clear the override.

**Params:** `title` (string, optional). Empty string is treated as "clear".

**Returns:** `{ ok: true, title: string|null }`

### `instance.focus`

Bring this instance's window to the foreground. **Cross-instance focus is intentionally not supported here** — to focus a peer, connect to that peer's MCP port directly and call `instance.focus` there. Keeps the auth model simple (each instance only ever acts on itself with its own token).

**Params:** ignored.

**Returns:** `{ ok: true, note: "stub in v0.9; OS-level window raise scheduled for v0.10" }`

In v0.9 the actual window-raise side effect is a stub — the call returns `ok: true` so client code doesn't have to special-case it, but the OS-level raise is tracked as a v0.10 polish item.

---

## Server

Self-description methods. These are the calls an agent makes first, before doing anything else, to figure out what it's connected to.

### `server.info`

Server identity. Static, doesn't reach into the mux.

**Params:** none.

**Returns:** `{ name: "Unterm MCP Server", version, engine: "Unterm (WezTerm)", protocol: "json-rpc-2.0" }`

### `server.health`

Health probe — checks the mux is available and reads a few stats out of it. Returns `status: "degraded"` if the mux is not yet up (rare; only happens during startup).

**Params:** none.

**Returns:** `{ status: "ok"|"degraded", engine, mcp: { bind, port, auth }, mux: { available, pane_count }, terminal: { initial_cols, initial_rows, color_scheme, term } }`

Note: the `mcp.port` field in the response is the *preferred* port (`19876`), not the actually-bound one. To get the actually-bound port, read `~/.unterm/server.json` or `instance.info`.

### `server.capabilities`

Machine-readable capability map — the canonical source of truth for "what method namespaces does this server support".

**Params:** none.

**Returns:** an object with one key per namespace, each value a list of fully-qualified method names. Used by `selftest.run` and by clients that want to feature-detect at runtime.

```json
{
  "session": ["session.list", "session.create", ...],
  "exec": ["exec.run", "exec.send", "exec.run_wait", "exec.status", "exec.cancel", "signal.send"],
  "screen": [...],
  "workspace": [...],
  "capture": [...],
  "proxy": [...],
  "governance": ["policy.set", "policy.check", "server.info", "server.health", "server.capabilities", "selftest.run"],
  "system": ["system.info", "system.launch_admin"],
  "instance": [...]
}
```

The `governance` umbrella in `server.capabilities` covers methods that don't have their own namespace (`policy.*`, `server.*`, `selftest.run`). Don't read too much into the grouping — it's a reporting structure, not a wire-level distinction.

---

## System

OS-level introspection and elevation.

### `system.info`

Process and platform metadata.

**Params:** none.

**Returns:** `{ name: "Unterm", version, engine: "Unterm (WezTerm)", platform, arch, active_sessions, hostname }`

`platform` is `std::env::consts::OS` (`"macos"`, `"linux"`, `"windows"`); `arch` is `std::env::consts::ARCH` (`"x86_64"`, `"aarch64"`).

### `system.launch_admin` (Windows only)

Spawn a fresh elevated Unterm window via PowerShell `Start-Process -Verb RunAs`. UAC prompt fires; user has to consent.

**Params:**

| Name | Type | Required | Description |
|---|---|---|---|
| `dry_run` | bool | no | If `true`, return the would-be command without executing |
| `shell` | string | no | `"powershell"`, `"pwsh"`, `"powershell7"`, etc. — picks which shell the elevated session runs |

**Returns:** `{ status: "launched"|"dry_run", requires_uac: true, command: [...] }`

On non-Windows platforms, returns `-32603` `"Administrator launch is only supported on Windows"`. The `selftest.run` self-check uses `dry_run: true` so it doesn't actually trigger UAC.

---

## Policy

Optional command-execution policy applied to `exec.run` and `exec.run_wait`. Disabled by default.

### `policy.set`

Set the policy. Replaces any previously set policy wholesale.

**Params:** the params object IS the policy:

| Name | Type | Required | Description |
|---|---|---|---|
| `enabled` | bool | yes | Whether to enforce the policy |
| `blocked_patterns` | string[] | yes | Substrings; any match blocks |
| `allowed_patterns` | string[] | yes | Stored but not currently enforced |

**Returns:** `{ set: true }`

```json
{"jsonrpc":"2.0","id":7,"method":"policy.set",
 "params":{"enabled":true,"blocked_patterns":["rm -rf /","sudo "],"allowed_patterns":[]}}
```

### `policy.check`

Dry-run a command against the current policy without executing it.

**Params:** `command` (string, required).

**Returns:** `{ allowed: bool, reason?: string }`

When enforcement is on and the command matches, returns `{ allowed: false, reason: "Blocked by pattern: <pattern>" }`. When the policy is disabled, always returns `{ allowed: true, reason: "Policy disabled" }`.

This is what `exec.run`/`exec.run_wait` call internally before sending input. If the check fails, those methods return `-32603` with `"Command blocked by policy: <pattern>"`.

---

## Selftest

### `selftest.run`

Run a battery of internal probes — mux availability, server health, capabilities listing, policy check, admin launch (dry-run on Windows; expected to fail on macOS/Linux), proxy status, window capture, and (if you pass `session_id`) per-pane checks.

**Params:** `session_id` (string, optional) — when present, adds `session.status`, `screen.text`, `screen.detect_errors`, recording-status checks scoped to that pane.

**Returns:** `{ ok: bool, checks: [{ name, ok, detail }, ...] }`

`ok` is true iff every check is true. Each `detail` is the full method response (or an `{ error }` object on failure). The `unterm-cli selftest` subcommand is a thin wrapper around this method.

```json
{"jsonrpc":"2.0","id":8,"method":"selftest.run","params":{"session_id":"0"}}
```

---

## Method index

Every method, alphabetical, with one-line descriptions. Use this as a flat lookup when you know the name and just want to confirm what it does.

| Method | Purpose |
|---|---|
| `auth.login` | Authenticate the connection with the token from `~/.unterm/instances/<id>.json` |
| `capture.clipboard` | Read the OS clipboard as text or PNG |
| `capture.screen` | Snapshot every pane's text plus a full-display PNG |
| `capture.select` | Falls back to `capture.screen` (interactive selection unavailable in headless mode) |
| `capture.window` | Snapshot one window by title or pid |
| `exec.cancel` | Send Ctrl+C to a pane |
| `exec.run` | Send a command + carriage return; return immediately |
| `exec.run_wait` | Send a command, inject a sentinel, poll until done; return captured output |
| `exec.send` | Alias for `session.input` |
| `exec.status` | Return `"idle"` or `"running"` based on foreground process name |
| `instance.focus` | Raise this instance's window to the foreground (stub on v0.9) |
| `instance.info` | This instance's own metadata, including `auth_token` |
| `instance.list` | Enumerate every live Unterm instance on this machine |
| `instance.set_title` | Pin a custom display title for this instance |
| `orchestrate.broadcast` | Send the same command to multiple panes |
| `orchestrate.launch` | `session.create` + initial command |
| `orchestrate.wait` | Poll a pane's text for a substring with a timeout |
| `policy.check` | Dry-run a command against the current policy |
| `policy.set` | Replace the command-execution policy |
| `proxy.configure` | Write a full proxy config in one call |
| `proxy.disable` | Turn the proxy off |
| `proxy.env` | Resolve proxy state to env-var form |
| `proxy.nodes` | List configured proxy nodes |
| `proxy.speedtest` | TCP-probe one node or all of them; persist latencies |
| `proxy.status` | Current proxy state plus reachability probe |
| `proxy.switch` | Activate one of the configured nodes by name |
| `screen.cursor` | Cursor position and shape |
| `screen.detect_errors` | Hardcoded error-pattern scan over the visible viewport |
| `screen.read` | Visible viewport with absolute row indices |
| `screen.scroll` | Read an absolute slice of scrollback |
| `screen.search` | Substring search across viewport + scrollback |
| `screen.text` | Visible viewport as a flat `lines[]` |
| `selftest.run` | Run an internal battery of probes |
| `server.capabilities` | Machine-readable namespace → method-list map |
| `server.health` | Health probe + mux/terminal stats |
| `server.info` | Server name, version, engine, protocol |
| `session.audit_log` | Read the in-memory audit log of mutating calls |
| `session.create` | Spawn a new pane in the active window |
| `session.cwd` | Get the pane's current working directory |
| `session.destroy` | Kill the pane |
| `session.env` | Stub: env-var read not supported in this build |
| `session.export_markdown` | One-off render of pane scrollback to redacted markdown |
| `session.get` | Full pane state (alias `session.status`) |
| `session.history` | Trailing N lines of scrollback as `entries[]` |
| `session.idle` | True if foreground process looks like a shell |
| `session.input` | Write raw bytes into pane stdin (alias `exec.send`) |
| `session.list` | Enumerate every live pane |
| `session.recording_attach_trace` | Associate an external trace id with a live recording |
| `session.recording_list` | Enumerate completed recordings on disk |
| `session.recording_read` | Read one recording's rendered markdown |
| `session.recording_start` | Begin recording a pane to redacted markdown |
| `session.recording_status` | Non-mutating: is this pane being recorded? |
| `session.recording_stop` | Finish, render the markdown, return paths and counts |
| `session.resize` | Resize the pane's pty |
| `session.set_env` | Stub: env-var write not supported in this build |
| `session.status` | Alias for `session.get` |
| `signal.send` | Send a control signal as a control character |
| `system.info` | Process and platform metadata |
| `system.launch_admin` | Spawn an elevated Unterm (Windows only) |
| `workspace.list` | Enumerate saved workspace names |
| `workspace.restore` | Read back a saved workspace (does not recreate panes) |
| `workspace.save` | Snapshot the current set of panes |

That's 60 methods plus `auth.login`. If you find a method in the codebase that isn't listed here, file an issue — the dispatch table at `wezterm-gui/src/mcp/handler.rs` is the source of truth and this page should track it.
