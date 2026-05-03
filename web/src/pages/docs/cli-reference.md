---
layout: ../../layouts/Doc.astro
title: unterm-cli reference
subtitle: Every subcommand and flag of the unterm-cli binary, with cron / CI / pipeline examples. Pipe --json through anywhere downstream that wants raw JSON-RPC.
kicker: Docs / CLI reference
date: 2026-05-03
---

## Connection model

`unterm-cli` is a thin JSON-RPC client. It does almost nothing on its own — every subcommand opens a TCP connection to the running Unterm GUI's MCP server, completes an `auth.login` handshake, and forwards the call.

When the GUI starts it writes `~/.unterm/server.json` with three fields:

```json
{
  "auth_token": "<uuid>",
  "mcp_port": 19876,
  "http_port": 19877
}
```

The CLI reads that file on every invocation and dials `127.0.0.1:<mcp_port>`. The token is per-launch — it rotates whenever the GUI restarts, and the file is written `0600` so other users on the host cannot read it. There is a fallback to the legacy `~/.unterm/auth_token` + port `19876` for builds that pre-date the multi-instance work, but new builds always use `server.json`.

A few consequences worth knowing before you wire scripts:

- **The GUI must be running.** No GUI, no MCP, no CLI. The error you'll get is `unterm GUI is not running — open Unterm.app to start the MCP server, or run 'unterm start' first`. Cron jobs that fire at boot should depend on the launch agent that owns Unterm.
- **Everything is local.** Both servers bind `127.0.0.1` only. Nothing on the LAN can reach them. There is no telemetry; the CLI never phones home.
- **The CLI is exactly the MCP surface, no more.** If a method exists on MCP, it's either reachable from the CLI today or trivially exposable. There is no parallel business logic — `unterm-cli` is a delivery shim.
- **Multi-instance / `active.json` is not yet wired.** Today there is one server per host; the CLI does not take an `--instance` flag. If you launch two Unterm windows on the same host, the second one's MCP server overwrites `server.json` and the CLI follows it.

The wire format is line-delimited JSON-RPC 2.0 — one request per line, one response per line. If you ever need to bypass the CLI and talk to MCP directly (Python, Node, curl-with-netcat, whatever), the protocol is documented in `wezterm-gui/src/mcp/server.rs`.

## Global flags

These are accepted on every subcommand because they're declared `global = true` on the top-level `clap` parser:

| Flag | Purpose |
|---|---|
| `--json` | Print the raw JSON-RPC `result` payload instead of the human-formatted table. Recognised by `proxy`, `theme`, `session`, `sessions`, `screenshot`, `lang`. Ignored by `settings open` (that command never round-trips through MCP). |
| `--lang <code>` | Force the CLI's interface locale for this single invocation. Does not write to `~/.unterm/lang.json`. Useful in scripts that need stable English output regardless of how the user has configured the GUI. Codes: `en-US`, `zh-CN`, `zh-TW`, `ja-JP`, `ko-KR`, `de-DE`, `fr-FR`, `it-IT`, `hi-IN`. |
| `-h`, `--help` | Print help for the current subcommand level. |
| `-V`, `--version` | Print the binary version (matches the GUI build, e.g. `unterm-cli 20260503-201120-8ceb3f23`). |

There are also a handful of inherited flags from the base `wezterm` CLI (`--skip-config`, `--config-file`, `--config name=value`) — these only matter for the GUI-launching subcommands (`start`, `ssh`, `connect`) and are no-ops for the MCP commands documented here.

The `--json` flag is the one to remember. Everything below has `--json` examples next to the human ones because that is how you should be driving the CLI from a script.

## proxy

Read or change the system-wide proxy state. The shape mirrors the GUI's Settings → Proxy panel: there's a global on/off, a `mode` (`auto`/`manual`/`off`), an HTTP and a SOCKS endpoint, an optional list of named "nodes" you can switch between, and a `no_proxy` exclusion list.

```text
unterm-cli proxy status
unterm-cli proxy nodes
unterm-cli proxy switch <NAME>
unterm-cli proxy disable
unterm-cli proxy env
```

### `proxy status`

```sh
$ unterm-cli proxy status
Proxy: ON
Mode:  auto
HTTP:  http://127.0.0.1:7897
SOCKS: socks5://127.0.0.1:7897
Current node: (none)
Node count: 0
No-proxy: 127.0.0.1,192.168.0.0/16,...,localhost,*.local,<local>
```

```sh
$ unterm-cli --json proxy status
{
  "current_node": null,
  "enabled": true,
  "health": {
    "hint": "",
    "reachable": true,
    "source": "manual",
    "url": "http://127.0.0.1:7897"
  },
  "http_proxy": "http://127.0.0.1:7897",
  "mode": "auto",
  "no_proxy": "127.0.0.1,...,<local>",
  "node_count": 0,
  "socks_proxy": "socks5://127.0.0.1:7897"
}
```

The `health` block is what the GUI uses to render the green/red dot in the proxy chip — `reachable: false` means the upstream proxy didn't answer the last heartbeat. Take it as a hint, not a guarantee; the heartbeat is cheap and async.

### `proxy nodes`

Lists named nodes from your proxy config. The active node, if any, is marked with `*`.

```sh
$ unterm-cli proxy nodes
   NAME                     URL
*  cn-shanghai              http://127.0.0.1:7897
   us-east-tunnel           http://10.0.0.5:8118
```

When no nodes are configured the human formatter prints `(no proxy nodes configured)`. The JSON form returns an empty `nodes` array — easier to feed to `jq`.

### `proxy switch <NAME>`

Sets the current node. The argument is the node `name` (not the URL). The MCP server actually accepts `node_name` on the wire; the CLI translates the surface for you. Example:

```sh
$ unterm-cli proxy switch us-east-tunnel
Switched: true
Current node: us-east-tunnel
HTTP: http://10.0.0.5:8118
```

Switching also flips `enabled = true`. If you've previously `disable`d the proxy, `switch` is the easy way back on.

### `proxy disable`

Hard off. Sets `mode = off`, clears `enabled`. Re-enabling without losing your config is what `proxy switch` is for, since `disable` doesn't drop the node list — it just deactivates the global flag.

```sh
$ unterm-cli proxy disable
Proxy disabled.
```

### `proxy env`

Emits `export` lines for the current proxy as POSIX shell. If the proxy is off, prints a comment instead of fake env vars.

```sh
$ unterm-cli proxy env
export ALL_PROXY=socks5://127.0.0.1:7897
export HTTPS_PROXY=http://127.0.0.1:7897
export HTTP_PROXY=http://127.0.0.1:7897
export NO_PROXY='127.0.0.1,...,<local>'
```

The output is shell-quoted: values that contain anything outside `[A-Za-z0-9:/.,_=-]` get wrapped in single quotes, with embedded `'` escaped. Safe to `eval`.

**Real-world use case** — drop this in your `~/.zshrc` or a project `direnv` `.envrc` so any new shell inherits whatever proxy Unterm is currently using:

```sh
# ~/.zshrc
if command -v unterm-cli >/dev/null 2>&1; then
  eval "$(unterm-cli proxy env 2>/dev/null)"
fi
```

When you flip proxies in Unterm's GUI, you don't have to restart shells — open a new tab and the new shell picks it up. For long-lived shells that need re-sync, a simple alias `alias rsp='eval "$(unterm-cli proxy env)"'` is enough.

## session

Operates on a single live pane. "Session" here means one terminal tab/pane in the running GUI. The MCP method names are `session.list`, `session.recording_start/stop/status`, and `session.export_markdown`.

```text
unterm-cli session list
unterm-cli session record start [--id <ID>]
unterm-cli session record stop  [--id <ID>]
unterm-cli session record status [--id <ID>]
unterm-cli session export       [--id <ID>] [-o FILE]
```

When `--id` is omitted on any `record` or `export` subcommand, the CLI auto-resolves it to the first pane returned by `session.list`. Convenient if you only have one tab open; brittle if you have several. Pass `--id` explicitly in scripts.

### `session list`

```sh
$ unterm-cli session list
ID    COLS   ROWS   SHELL      TITLE
0     191    77     unknown    ✳ Claude Code
2     171    77     unknown    ⠂ Check current project progress
```

```sh
$ unterm-cli --json session list
{
  "sessions": [
    {
      "cols": 191, "rows": 77,
      "cursor": { "visible": false, "x": 0, "y": 7812 },
      "domain_id": 0, "id": 0, "is_dead": false,
      "shell": {
        "cwd": null,
        "process_name": "/Users/alexlee/.local/share/claude/versions/2.1.126",
        "shell_type": "unknown"
      },
      "title": "✳ Claude Code"
    }
  ]
}
```

The IDs are stable for the lifetime of a pane and monotonically increasing. They are *not* reused after a pane closes, so a script that snapshots IDs once and replays them later is safe — at worst you'll get "Session 7 not found" rather than acting on the wrong pane.

### `session record start`

Begins a redacted markdown recording of a pane. Returns a UUID (`session_id`) and the on-disk paths the recording will land at.

```sh
$ unterm-cli session record start --id 0
Session id: 8dee59d3-0e21-4ebf-a8cf-a2c356b53b70
Log path: /Users/alexlee/.unterm/sessions/_orphan/2026-05-03/tab-0-221510.log
Markdown (on stop): /Users/alexlee/.unterm/sessions/_orphan/2026-05-03/tab-0-221510.md
```

If the pane has a project cwd, recordings land under `<cwd>/.unterm/sessions/<date>/` instead of `~/.unterm/sessions/_orphan/<date>/`. The fallback is what you get when the pane has no detectable project root.

### `session record stop` / `record status`

`stop` finalises the markdown (no further blocks captured), prints summary stats, and is idempotent — calling it on a non-recording pane prints a benign "not recording" message rather than failing.

```sh
$ unterm-cli session record stop --id 0
Session id: 8dee59d3-0e21-4ebf-a8cf-a2c356b53b70
Block count: 0
Markdown: /Users/alexlee/.unterm/sessions/_orphan/2026-05-03/tab-0-221510.md
Exit reason: recording_stopped
```

`status` is read-only and useful for "was I already recording?" guards in scripts:

```sh
$ unterm-cli --json session record status --id 0
{ "enabled": false }
```

### `session export`

Snapshots a pane's accumulated block log to markdown without stopping recording (or even requiring recording to be active — the block buffer is always populated when OSC 133 is in play). Two flag modes:

- No `-o`: MCP picks the destination, the path is printed.
- `-o FILE`: the CLI passes the path through to MCP, and additionally copies the file to `FILE` on the local filesystem if MCP wrote elsewhere. End state: `FILE` always exists at the path you asked for.

```sh
$ unterm-cli session export --id 0 -o /tmp/snapshot.md
/tmp/snapshot.md
```

**Real-world use case** — a "git pre-push" hook that exports the last 100 blocks of your build pane and attaches them to the commit message:

```sh
# .git/hooks/pre-push
PANE_ID=$(unterm-cli --json session list | jq '.sessions[] | select(.title|test("build|ci")) | .id' | head -1)
[ -z "$PANE_ID" ] && exit 0
unterm-cli session export --id "$PANE_ID" -o ".git/last-build.md"
```

## sessions

Browse the persistent recording archive on disk (the markdown files written by `session record stop` and friends). The MCP-side methods are `session.recording_list` and `session.recording_read`.

```text
unterm-cli sessions list [--project <SLUG>]
unterm-cli sessions read <SESSION_ID>
```

### `sessions list`

```sh
$ unterm-cli sessions list
SESSION_ID                             BLOCKS STARTED                          PROJECT
95397675-cb95-4116-9c8e-64a0c32ce927   1      2026-04-30T22:03:26.889127+08:00 alexlee
0ec12e9d-a9ae-43f6-a654-4b484789727e   1      2026-05-01T09:40:29.205705+08:00 unterm
8ae4a612-08e2-4ba5-af1f-d815c80abdd2   1      2026-05-01T09:54:38.014989+08:00 unterm
```

Filter to a project:

```sh
$ unterm-cli sessions list --project unterm
```

The "project slug" is the basename of the directory the recording originated from (or `_orphan` for recordings without a detectable project). It's matched as an exact string, not a glob.

JSON form returns an array (not an object with a `sessions` key — note the asymmetry vs `session.list`):

```sh
$ unterm-cli --json sessions list | jq '.[0]'
{
  "block_count": 1,
  "bytes_raw": 4136,
  "ended_at": "2026-04-30T14:03:29.296032+00:00",
  "log_path": "/Users/alexlee/.unterm/sessions/alexlee/2026-04-30/tab-0-220326.log",
  "md_path": "/Users/alexlee/.unterm/sessions/alexlee/2026-04-30/tab-0-220326.md",
  "project_path": "/Users/alexlee/",
  "project_slug": "alexlee",
  "started_at": "2026-04-30T22:03:26.889127+08:00",
  "tab_id": 0,
  "unterm_session_id": "95397675-cb95-4116-9c8e-64a0c32ce927"
}
```

### `sessions read <SESSION_ID>`

Streams the recorded markdown to stdout. The argument is the UUID from `sessions list`, not the path. Pipe it however you like.

```sh
$ unterm-cli sessions read 95397675-cb95-4116-9c8e-64a0c32ce927 | head
---
unterm_session_id: 95397675-cb95-4116-9c8e-64a0c32ce927
tab_id: 0
project_path: /Users/alexlee/
project_slug: alexlee
shell: /bin/sh
hostname: 192.168.5.7
unterm_version: 20260502-121851-b3680e89
started_at: 2026-04-30T22:03:26.889127+08:00
ended_at: 2026-04-30T14:03:29.296032+00:00
```

The frontmatter is YAML; the body is fenced markdown blocks, one per OSC 133 prompt. Tokens, GitHub PATs, AWS keys, and 40+ char base64/hex strings are masked at recording time.

**Real-world use case** — feed a recent recording to a model for review:

```sh
LAST=$(unterm-cli --json sessions list --project unterm | jq -r '.[-1].unterm_session_id')
unterm-cli sessions read "$LAST" | claude -p "summarise what I did in this session"
```

## settings

Open the Web Settings UI in your default browser. This subcommand does *not* hit MCP — it reads `server.json` directly to find the `http_port` and shells out to `open` / `xdg-open` / `cmd /C start`.

```text
unterm-cli settings open [--print-only]
```

| Flag | Purpose |
|---|---|
| `--print-only` | Print the URL and exit; don't launch a browser. |

```sh
$ unterm-cli settings open --print-only
http://127.0.0.1:19877
```

```sh
$ unterm-cli settings open
http://127.0.0.1:19877
# (browser tab opens)
```

The URL also points at a static SPA at `/`, plus a small REST surface for the same operations as MCP — handy if you want to drive Unterm from JavaScript in a browser without speaking JSON-RPC.

## screenshot

Capture the screen and save the PNG. Backed by `capture.screen` over MCP.

```text
unterm-cli screenshot [--include-window] [-o FILE]
```

| Flag | Purpose |
|---|---|
| `--include-window` | Include Unterm's own window in the capture. Default: pass `include_window=false` to MCP, which the GUI honours best-effort (`screencapture` cannot literally exclude one window, so on macOS this currently maps to "capture full screen anyway" — treat the flag as a hint). |
| `-o`, `--output <FILE>` | Local path to write the PNG to. The CLI copies from the MCP-side path if MCP writes elsewhere. End state: `FILE` exists. |

```sh
$ unterm-cli screenshot --output /tmp/cap.png
/tmp/cap.png
```

```sh
$ unterm-cli --json screenshot | jq '.image'
{
  "path": "/Users/alexlee/.unterm/screenshots/screen_20260503_221502_301.png",
  "width": 2560,
  "height": 1440,
  "left": 0,
  "top": 0
}
```

The `--json` form additionally returns `captures[]` with the on-screen text content of every visible Unterm pane — handy if you want to capture both the pixels and the textual state in one round trip. Set `include_base64=true` directly via MCP if you need the image inline rather than a path; the CLI does not currently surface that flag.

**Real-world use case** — a CI step that snaps the screen of a self-hosted runner whenever the build fails, for human triage:

```sh
# .github/scripts/on-failure.sh
unterm-cli screenshot --include-window -o "/tmp/ci-failure-${GITHUB_RUN_ID}.png"
gh run upload-artifact "/tmp/ci-failure-${GITHUB_RUN_ID}.png" --name screenshot
```

## theme

List preset themes or switch to one. Backed by `~/.unterm/theme.json`, watched by the running GUI — no MCP round-trip; the CLI writes the file and the GUI picks it up.

```text
unterm-cli theme list
unterm-cli theme switch <NAME>   # alias: theme set
```

The four built-in presets are `standard`, `midnight`, `daylight`, `classic`.

```sh
$ unterm-cli theme list
Active: classic

   ID         NAME           COLOR SCHEME                 DESCRIPTION
   standard   Standard       Catppuccin Mocha             Balanced dark terminal style
   midnight   Midnight       Tokyo Night                  Low-glare blue-black workspace
   daylight   Daylight       Builtin Solarized Light      Readable light mode for bright rooms
*  classic    Classic        Builtin Tango Dark           Plain high-contrast terminal colors
```

```sh
$ unterm-cli --json theme switch daylight
{
  "color_scheme": "Builtin Solarized Light",
  "id": "daylight",
  "name": "Daylight",
  "switched": true
}
```

Names are matched case-insensitively. Unknown names error out with a non-zero exit and a useful message ("Unknown theme 'foo'. Run …").

## lang

List, set, or print the active interface locale. Operates on `~/.unterm/lang.json` — also no MCP round-trip. Affects only the locale the CLI itself uses for human-formatted output and (after the GUI re-reads the file) the GUI's UI strings.

```text
unterm-cli lang list
unterm-cli lang set <CODE>
unterm-cli lang current
```

Supported codes: `en-US`, `zh-CN`, `zh-TW`, `ja-JP`, `ko-KR`, `de-DE`, `fr-FR`, `it-IT`, `hi-IN`.

```sh
$ unterm-cli lang list
    CODE     ACTIVE         NAME
*   en-US    *              English
    zh-CN                   简体中文
    zh-TW                   繁體中文
    ja-JP                   日本語
    ko-KR                   한국어
    de-DE                   Deutsch
    fr-FR                   Français
    it-IT                   Italiano
    hi-IN                   हिन्दी
```

```sh
$ unterm-cli --json lang current
{
  "code": "en-US",
  "name": "English"
}
```

`lang set` persists; the global `--lang <code>` flag does not. If you're scripting English-only output for downstream tools, prefer `--lang en-US` per-invocation rather than overwriting the user's preference.

## shell-completion

Emits a completion script for your shell. Sourceable.

```text
unterm-cli shell-completion --shell <bash|zsh|fish|elvish|powershell|fig>
```

```sh
$ unterm-cli shell-completion --shell zsh > "${fpath[1]}/_unterm-cli"
$ exec zsh
```

This is the same generator the upstream `wezterm` binary uses (`clap_complete`), which means the completions cover everything in `unterm-cli --help`, including the MCP subcommands documented above.

---

## Scripting cookbook

A few patterns that come up often. They all assume `unterm-cli` is on `PATH` (the installer drops it next to the GUI binary; `brew install --cask unterm` symlinks it).

### Wait for a long-running build, then notify

```sh
#!/usr/bin/env bash
# Block until pane $PANE_ID has been idle for 10s, then send a notification.
PANE_ID="${1:?usage: wait-then-notify <pane-id>}"

last_y=""
idle_start=""
while :; do
  cur_y=$(unterm-cli --json session list \
    | jq -r --argjson id "$PANE_ID" '.sessions[] | select(.id == $id) | .cursor.y')
  now=$(date +%s)
  if [ "$cur_y" = "$last_y" ]; then
    [ -z "$idle_start" ] && idle_start=$now
    if [ $((now - idle_start)) -ge 10 ]; then break; fi
  else
    idle_start=""
    last_y=$cur_y
  fi
  sleep 2
done

osascript -e 'display notification "build finished" with title "unterm"'
```

The cursor `y` value monotonically increases as a pane scrolls, so freezing it for N seconds is a cheap "is idle" proxy. For a stricter signal, pair this with `session export` and check the tail for a known terminator string.

### Capture a screenshot from a CI runner for the failure report

```sh
# scripts/on-test-failure.sh
mkdir -p artifacts
unterm-cli screenshot --include-window -o "artifacts/failure-$(date +%s).png"
unterm-cli --json session list \
  | jq '.sessions[] | {id, title, lines: .cursor.y}' \
  > artifacts/pane-state.json
```

Self-hosted runners that boot Unterm at startup get a full visual + textual snapshot every time a check fails. Both files are well under GitHub's artifact size limits.

### Auto-rotate session recordings nightly

```sh
# crontab: 30 3 * * *  /usr/local/bin/rotate-unterm-sessions.sh
#!/usr/bin/env bash
THRESHOLD=$(date -v-30d +%Y-%m-%dT%H:%M:%S 2>/dev/null || date -d '30 days ago' -Iseconds)

unterm-cli --json sessions list \
  | jq -r --arg t "$THRESHOLD" '.[] | select(.started_at < $t) | .md_path' \
  | while read -r path; do
      [ -f "$path" ] && gzip -9 "$path"
    done
```

A cron entry at 3:30am scans the archive, gzips anything older than 30 days. The recordings index in MCP doesn't auto-evict — it's expected that you bring your own retention policy.

### Switch theme based on time of day from cron

```sh
# crontab:
#   0 7 * * *  /usr/local/bin/unterm-cli theme switch daylight >/dev/null
#   0 19 * * * /usr/local/bin/unterm-cli theme switch midnight >/dev/null
```

That's it — `theme switch` writes `theme.json`, the running GUI's file watcher picks it up within the next frame, no extra plumbing.

For something fancier (latitude/longitude sunrise rather than wall-clock 7am), wrap a Python `astral` call around the same two CLI invocations.

### Drive a multi-pane lint dashboard

This pattern is the closest the CLI comes to "outer agent" territory — kick off the same command in every pane that matches a filter, then aggregate the tails.

```sh
#!/usr/bin/env bash
# lint-everything.sh — run `make lint` in every pane whose title contains "lint"
PANES=$(unterm-cli --json session list \
  | jq '[.sessions[] | select(.title | test("lint"; "i")) | .id]')

# session.send_text isn't yet wrapped by the CLI — direct MCP via netcat.
PORT=$(jq -r .mcp_port ~/.unterm/server.json)
TOKEN=$(jq -r .auth_token ~/.unterm/server.json)

# Use a small helper that speaks line-delimited JSON-RPC.
send() {
  python3 -c "
import json, socket, sys
s = socket.create_connection(('127.0.0.1', $PORT))
def call(m, p):
    s.sendall((json.dumps({'jsonrpc':'2.0','id':1,'method':m,'params':p}) + '\n').encode())
    return json.loads(s.makefile().readline())
print(call('auth.login', {'token': '$TOKEN'}))
print(call('$1', $2))
"
}

for id in $(echo "$PANES" | jq '.[]'); do
  send session.send_text "{\"id\": $id, \"text\": \"make lint\\n\"}"
done

# Wait then export tails.
sleep 30
mkdir -p /tmp/lint-report
for id in $(echo "$PANES" | jq '.[]'); do
  unterm-cli session export --id "$id" -o "/tmp/lint-report/pane-$id.md"
done
```

A future CLI release will wrap `session.send_text` and `session.read_tail` directly so you don't need the inline Python; the surface is already there on MCP.

### Snapshot pane state into git history

```sh
# Drop me in $PROJECT/.git/hooks/post-commit
PANE=$(unterm-cli --json session list | jq -r '.sessions[0].id')
DIR=".unterm/per-commit"
mkdir -p "$DIR"
unterm-cli session export --id "$PANE" -o "$DIR/$(git rev-parse --short HEAD).md"
git add "$DIR" 2>/dev/null
```

Every commit records the state of your active terminal as part of the project's `.unterm/` directory. Combined with `.unterm/sessions/...` recordings, you end up with a per-commit log of "what was I actually doing when I made this change". The redaction layer keeps tokens out of the markdown.

### Mirror proxy state into shell sessions

Worth repeating from earlier as a one-liner — the shell-init pattern that keeps every new shell aligned with the GUI:

```sh
# zsh
eval "$(unterm-cli proxy env 2>/dev/null)"
```

If `unterm-cli` isn't installed, the `eval` no-ops because there's no output. If the GUI isn't running, the CLI exits non-zero with no stdout, same effect. Cheap and safe to drop into init scripts.

---

## Exit codes

`unterm-cli` follows the standard convention: `0` on success, `1` on any error.

The Rust source is `wezterm/src/main.rs::run()`, which propagates an `anyhow::Error` from each subcommand up to `terminate_with_error()`, which calls `std::process::exit(1)`. There are no granular status codes today — every failure mode collapses to `1`. Distinguish them by message on stderr:

```sh
$ unterm-cli proxy switch nonexistent-node
ERROR  unterm_cli > MCP proxy.switch failed [-32603]: Proxy node 'nonexistent-node' not found; terminating
$ echo $?
1

$ unterm-cli session record start --id 99999
ERROR  unterm_cli > MCP session.recording_start failed [-32603]: Session 99999 not found; terminating
$ echo $?
1

$ unterm-cli lang set bogus-locale
ERROR  unterm_cli > unknown locale 'bogus-locale'. Run `unterm-cli lang list` to see options.; terminating
$ echo $?
1
```

The bracketed code in MCP errors (e.g. `[-32603]`) is the JSON-RPC error code — `-32603` is the spec's "internal error". For known constraint failures (locale, theme name, missing pane) the message is unambiguous. Script defensively:

```sh
if ! out=$(unterm-cli proxy status 2>&1); then
  case "$out" in
    *"GUI is not running"*) launchctl start com.unzooai.unterm; sleep 2; ;;
    *) echo "unterm-cli failed: $out" >&2; exit 1 ;;
  esac
fi
```

If you need machine-readable failure detail, use `--json` and inspect the resulting `error` field — but note that today the CLI translates JSON-RPC errors to `anyhow` errors before printing, so `--json` only formats the *success* result. For raw protocol-level errors, talk to MCP directly with the netcat / Python pattern above. That's the escape hatch when the CLI's surface isn't quite enough.

---

Source for everything described here lives at [github.com/unzooai/unterm](https://github.com/unzooai/unterm) under `wezterm/src/unterm_cli/`. Open issues / PRs there if a method exists on MCP that you'd like surfaced as a first-class CLI subcommand.
