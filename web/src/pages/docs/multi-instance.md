---
layout: ../../layouts/Doc.astro
title: Driving multiple Unterms from one agent
subtitle: NATO-phonetic instance names, the MCP discovery protocol, and four orchestration patterns for multi-window AI workflows.
kicker: Docs / Multi-instance
date: 2026-05-03
---

## Why this exists

One Unterm window covers the obvious case — an outer agent supervises the shell, the human watches over its shoulder. As soon as the agent has any ambition, that breaks down. A coding agent that touches three repos wants three terminals open in three different cwds. A reviewer agent that runs `cargo build` while the implementer agent edits source needs them in separate windows so the build output doesn't collide with whatever the implementer's typing. Anything resembling a router-and-workers pattern wants a fleet.

The trouble is, "open three Unterm windows" is trivial — the agent can just spawn three processes. The harder question is which one a given MCP request is hitting. Pre-v0.9 every Unterm wrote its port and auth token into `~/.unterm/server.json`, and whichever process won the race to write last got to be the canonical one. The other two were unreachable from outside.

Multi-instance fixes that. Every Unterm process now claims a stable, human-readable name (alpha, bravo, charlie…), drops a per-process metadata file under `~/.unterm/instances/<name>.json`, and keeps it there for as long as the process lives. Agents enumerate the directory, pick which instance they want by cwd or title or start time, and connect to that one's MCP port directly.

The director-worker pattern is the concrete motivating example. An outer Claude session acts as director: it reads a roadmap, picks a task, spawns an inner Claude in a fresh Unterm window pointed at the right repo, waits for the work to complete, then dispatches the next task. With a fleet of inner Claudes, the director can run three tasks in parallel — one in `unterm`, one in `solomd`, one in some downstream repo — and aggregate their results. None of that is possible if every MCP call lands on the same window.

This page covers the discovery protocol, the four orchestration patterns we've actually used, and a 40-line Python harness you can graft into your own agent.

## The NATO naming scheme

The 26 names are: `alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo sierra tango uniform victor whiskey xray yankee zulu`. They live in `NATO_NAMES` in `wezterm-gui/src/server_info.rs` and that order is the assignment order — every newly launched Unterm tries `alpha` first, falls back to the next free one.

Why a wordlist instead of UUIDs or Crockford Base32. The decision was made on 2026-05-02 and is locked: AI agents pronounce NATO names correctly when they read them aloud or write them into other prompts, humans can say them on a phone call ("could you look at bravo?"), and they fit cleanly into a window title. A base32 ID like `7H4K9N2P` is unique-but-mute — when an agent has to surface "switch to instance X" to a human, that human can't read X back into a different agent's input box without copy-paste. NATO names eliminate that friction.

Twenty-six names is more than enough for the dogfood case. If they're all simultaneously taken — running 27 Unterms isn't unheard of when you've parked some in tmux and forgot — the claim logic appends a digit: `alpha2`, `bravo2`, all the way through `zulu99`. The cap is 99 (`NATO×99` = 2,574 instance ceiling). If you hit it, something else is wrong.

The window title reflects the name automatically. Each Unterm window is titled `Unterm — <name> — <pane title>`, em-dash separated. So an agent can also pull the name out of the OS-level window list (NSWindow/AccessibilityUIElement on macOS, `xdotool getwindowname` on X11) and cross-reference with the instance file. The auto-title rule is locked — a user-supplied `format-window-title` Lua callback still wins, but the default for everyone who hasn't customized is the three-segment pattern.

## File layout

Everything lives under `~/.unterm/`. The relevant pieces:

```
~/.unterm/
  instances/
    alpha.json        ← per-process metadata, written on launch, deleted on exit
    bravo.json
    charlie.json
  active.json         ← pointer to the most recently launched live instance
  server.json         ← legacy compat mirror; same content as active.json
  auth_token          ← legacy compat mirror; same content as active.json:auth_token
```

Each `instances/<name>.json` is the source of truth for one running process. Here's a real one:

```json
{
  "id": "bravo",
  "mcp_port": 19877,
  "http_port": 19878,
  "auth_token": "f3c0a8e2-9b14-4e31-a7d5-2f1c9d8a4b6e",
  "pid": 84213,
  "started_at": "2026-05-03T11:42:08.137-07:00",
  "title": null,
  "cwd": "/Volumes/Dev/code/solomd",
  "version": "0.11.2",
  "platform": "macos"
}
```

The `mcp_port` may differ from the canonical `19876`. Each instance binds the preferred port first, then `19876+1 .. 19876+5`, then falls back to OS-assigned. The `auth_token` is fresh per launch — UUID v4, never persisted across runs. The `cwd` is best-effort, refreshed periodically by the foreground update loop; if you need ground truth, call `session.list` over MCP. The `title` is `null` when the auto-title applies, or a string when the user (or an agent) has pinned an override via `instance.set_title`.

`active.json` is a pointer. It exists for legacy single-instance agents that still read the old `server.json` path — that file's been there since v0.4 and we don't break it. It contains the full `InstanceInfo` of whichever instance is "active." The semantics are: `active.json` is updated only when the previous active dies. Quitting bravo while alpha is also live transfers the active pointer to alpha. Launching a third window when bravo is alive does **not** make the new window active — bravo retains the pointer. Focus events do not update it.

That's a deliberate trade-off. The user's foreground window may not match `active.json` for short windows of time. The benefit is no per-focus disk write, no thrash when alt-tabbing, and a stable contract that legacy agents can rely on. Multi-instance agents don't read `active.json` at all — they read `instances/` directly and pick their own target.

`server.json` is identical content to `active.json`, kept because v0.4-era integrations expect it. The `auth_token` file is the auth-token field of the active instance, written separately for shell scripts that just want `cat ~/.unterm/auth_token | curl -H "Authorization: Bearer $(cat -)"`-style usage.

## The discovery protocol

The full sequence an agent follows to talk to a specific Unterm:

1. Read every `*.json` under `~/.unterm/instances/`.
2. Parse each. Drop entries whose `pid` is no longer alive — the storage layer also does this on its own next time `instance.list` is called, but reading directly is fine.
3. Pick one by whatever criterion you care about: most recent `started_at`, matching `cwd`, custom `title` you set earlier, lowest NATO name (alpha-first).
4. Open a TCP connection to `127.0.0.1:<mcp_port>` of the chosen instance.
5. Send a JSON-RPC `auth.login` with the instance's `auth_token`. The server replies `{"status": "ok"}` on success or `-32001 Invalid auth token` on mismatch.
6. Send any other method you want. Until you authenticate, every method other than `auth.login` returns `-32002 Not authenticated`.

The wire format is line-delimited JSON-RPC 2.0: each request is a single-line JSON object terminated with `\n`, each response the same. Here it is in Node:

```js
import { connect } from "node:net"
import { readdir, readFile } from "node:fs/promises"
import { homedir } from "node:os"
import { join } from "node:path"

async function listInstances() {
  const dir = join(homedir(), ".unterm", "instances")
  const files = await readdir(dir)
  const out = []
  for (const f of files) {
    if (!f.endsWith(".json")) continue
    const raw = await readFile(join(dir, f), "utf8")
    out.push(JSON.parse(raw))
  }
  return out
}

async function rpc(port, token, method, params = {}) {
  return new Promise((resolve, reject) => {
    const sock = connect(port, "127.0.0.1")
    let buf = "", queue = []
    sock.on("data", chunk => {
      buf += chunk
      let nl
      while ((nl = buf.indexOf("\n")) >= 0) {
        const line = buf.slice(0, nl); buf = buf.slice(nl + 1)
        const cb = queue.shift(); if (cb) cb(JSON.parse(line))
      }
    })
    sock.on("error", reject)
    sock.on("connect", async () => {
      const send = (m, p) => new Promise(res => {
        queue.push(res)
        sock.write(JSON.stringify({ jsonrpc: "2.0", id: 1, method: m, params: p }) + "\n")
      })
      const auth = await send("auth.login", { token })
      if (auth.error) { sock.end(); return reject(new Error(auth.error.message)) }
      const result = await send(method, params)
      sock.end()
      result.error ? reject(new Error(result.error.message)) : resolve(result.result)
    })
  })
}

const all = await listInstances()
const target = all.find(i => i.cwd?.endsWith("/unterm"))
const sessions = await rpc(target.mcp_port, target.auth_token, "session.list")
console.log(sessions)
```

The PID-liveness check is doing a `kill(pid, 0)` on Unix and `OpenProcess` + `GetExitCodeProcess` on Windows. Both are cheap. If you're enumerating in a tight loop, the storage layer's `list_live_instances()` does the same sweep and deletes stale files as a side effect — so calling `instance.list` against any live instance is a fine way to clean up the directory.

## MCP methods reference

Four methods, all under the `instance.` namespace.

### `instance.list`

Enumerate every live Unterm process on this machine.

Request:

```json
{"jsonrpc":"2.0","id":1,"method":"instance.list","params":{}}
```

Response:

```json
{"jsonrpc":"2.0","id":1,"result":{"instances":[
  {"id":"alpha","pid":84120,"started_at":"2026-05-03T11:38:02-07:00",
   "mcp_port":19876,"http_port":19877,"title":null,
   "cwd":"/Volumes/Dev/code/unterm","version":"0.11.2","platform":"macos"},
  {"id":"bravo","pid":84213,"started_at":"2026-05-03T11:42:08-07:00",
   "mcp_port":19877,"http_port":19878,"title":"[reviewer]",
   "cwd":"/Volumes/Dev/code/solomd","version":"0.11.2","platform":"macos"}
]}}
```

Note `auth_token` is **not** included in `instance.list` — the listing endpoint reveals which ports exist, not how to authenticate to them. To get the token you read the file directly (you have filesystem access if you have `~/.unterm/instances/` access) or call `instance.info` against an already-authenticated connection.

### `instance.info`

Return the metadata of *this* instance — the one you're connected to.

Request:

```json
{"jsonrpc":"2.0","id":2,"method":"instance.info","params":{}}
```

Response:

```json
{"jsonrpc":"2.0","id":2,"result":{
  "id":"alpha","pid":84120,"started_at":"2026-05-03T11:38:02-07:00",
  "mcp_port":19876,"http_port":19877,
  "auth_token":"a1b2c3d4-...",
  "title":null,"cwd":"/Volumes/Dev/code/unterm",
  "version":"0.11.2","platform":"macos"
}}
```

Useful as a sanity check after `auth.login`. Confirm you're on the instance you thought you were on.

### `instance.set_title`

Pin a custom display label for this instance. Overrides the auto-derived `Unterm — <name> — <project>` window title; the `<name>` segment becomes whatever string you pass. Pass `null` (or omit `title`) to clear the override.

Request:

```json
{"jsonrpc":"2.0","id":3,"method":"instance.set_title","params":{"title":"[claude-A]"}}
```

Response:

```json
{"jsonrpc":"2.0","id":3,"result":{"ok":true,"title":"[claude-A]"}}
```

After this call, the window title becomes `Unterm — [claude-A] — <pane title>`, and the same string appears in `instance.list` output as the `title` field of this instance. Other agents can see "[claude-A]" and route around it.

### `instance.focus`

Bring this instance's window to the foreground. To focus a peer, you connect to that peer's MCP port — there's no "focus instance bravo from inside alpha" cross-call; each instance only ever acts on itself with its own token.

Request:

```json
{"jsonrpc":"2.0","id":4,"method":"instance.focus","params":{}}
```

Response:

```json
{"jsonrpc":"2.0","id":4,"result":{"ok":true,"note":"stub in v0.9..."}}
```

The current implementation returns `ok: true` so agents can rely on the call shape, but the OS-level window-raise side effect is a stub. Scheduled for v0.13+. Workaround for now: the agent can spawn a pane via `session.create` in the target instance, which triggers WezTerm's existing focus behavior, or trip the user's `Cmd-` ` shortcut. Or just rely on the user to alt-tab when the title says it's the right one.

## Four orchestration patterns

The patterns we've actually used. Every code example assumes the JSON-RPC helper from the discovery protocol section above (`rpc(port, token, method, params)`).

### Pattern 1 — One agent, one window

The legacy case. Single-instance agents that pre-date v0.9 read `~/.unterm/server.json`, get the active instance's port and token, and connect. They don't know multi-instance exists. They keep working unchanged because `server.json` always mirrors whichever instance is currently active.

```js
const cfg = JSON.parse(await readFile(join(homedir(), ".unterm", "server.json"), "utf8"))
const sessions = await rpc(cfg.mcp_port, cfg.auth_token, "session.list")
```

The trade-off: if the user has three Unterms open and the agent's writing this way, it will land on whichever was active first. That's still deterministic — `active.json` only updates on previous-active-death, so during a session the target instance doesn't shift around under the agent's feet.

### Pattern 2 — One agent, many windows

The agent has multiple Unterms open and wants to dispatch into the right one based on context. Common scenario: a workspace-aware coding agent that sees the user is asking about repo X, finds the Unterm whose cwd is X, and runs commands there.

```js
const all = await listInstances()
const target = all.find(i => i.cwd && i.cwd.includes("/solomd"))
if (!target) throw new Error("no Unterm open in solomd")
await rpc(target.mcp_port, target.auth_token, "session.send_text",
  { id: 0, text: "cargo test --lib\n" })
```

Selection criteria worth considering: most-recent `started_at` (newest window wins), exact `cwd` match, custom `title` set by an earlier `instance.set_title`. The agent picks the criterion that matches its own product semantics.

### Pattern 3 — Many agents, one window each

Multiple agents running concurrently, each owning exactly one Unterm. To avoid stepping on each other, each agent claims a window at startup by setting a unique title:

```js
// Agent startup. Pick an unclaimed window.
const all = await listInstances()
const claimed = new Set(all.map(i => i.title).filter(Boolean))
const target = all.find(i => !i.title)
if (!target) throw new Error("all windows claimed")

await rpc(target.mcp_port, target.auth_token, "instance.set_title",
  { title: `[agent-${process.env.AGENT_ID}]` })

// From now on, this agent only ever talks to `target`. Other agents
// looking at instance.list see the [agent-…] tag and skip past.
const myPort = target.mcp_port
const myToken = target.auth_token
```

This is the cleanest pattern for human-supervised multi-agent setups. The window title shows `Unterm — [agent-1] — <pane>`, the human can see at a glance which window is which agent, and the title also appears in `instance.list` so peer agents auto-route around claimed windows.

A concurrency note: two agents can race on `instance.set_title` for the same window. The atomic file write means the last writer wins, so if you're worried about it, after setting your title call `instance.info` and verify the title that came back matches what you set. If not, the other agent claimed it first; pick a different window.

### Pattern 4 — Router agent dispatches a fleet

The director-worker case from the opening section. One outer agent spawns N Unterm windows (each will claim its own NATO name), tags each with a role, and dispatches per-task work. The outer agent never runs commands itself; it only orchestrates.

```js
import { spawn } from "node:child_process"

// 1. Spawn N fresh Unterm processes. They each claim their own NATO name.
const repos = ["unterm", "solomd", "unflick"]
for (const r of repos) {
  spawn("open", ["-na", "Unterm", "--args", "--cwd", `/Volumes/Dev/code/${r}`],
    { detached: true, stdio: "ignore" })
}

// 2. Wait for them to register their instance files.
await new Promise(r => setTimeout(r, 2000))

// 3. Discover and tag.
const all = await listInstances()
const newOnes = all.filter(i => Date.parse(i.started_at) > Date.now() - 5000)
const fleet = {}
for (let n = 0; n < newOnes.length; n++) {
  const i = newOnes[n]
  const role = `worker-${repos[n]}`
  await rpc(i.mcp_port, i.auth_token, "instance.set_title", { title: `[${role}]` })
  fleet[role] = i
}

// 4. Dispatch per-worker tasks.
async function dispatch(role, task) {
  const i = fleet[role]
  const session = await rpc(i.mcp_port, i.auth_token, "session.create", { cwd: i.cwd })
  await rpc(i.mcp_port, i.auth_token, "session.send_text",
    { id: session.id, text: `claude code -p "${task}"\n` })
  return session.id
}

await Promise.all([
  dispatch("worker-unterm", "fix the v0.11 multi-instance race condition"),
  dispatch("worker-solomd", "rebase main onto upstream"),
  dispatch("worker-unflick", "ship the dropped-frame counter to settings"),
])

// 5. Poll each worker, aggregate.
async function status(role) {
  const i = fleet[role]
  return rpc(i.mcp_port, i.auth_token, "session.list")
}
```

The router has the strategic picture; each worker has its own keyboard. The router decides which task lands where based on cwd / repo / capacity. Step 5 typically runs in a polling loop with `session.read_tail` to detect completion markers in each worker's pane.

## Lifecycle gotchas

A handful of edge cases worth knowing about.

**Crashes leave stale files.** If an Unterm process is hard-killed (SIGKILL, panic, OOM), the cleanup code in `shutdown()` doesn't run, and `~/.unterm/instances/<name>.json` is left on disk. The next process to call `list_live_instances()` does a PID-liveness sweep — for each file, it checks whether the recorded `pid` is still a running process. If not, it deletes the file. So stale entries are self-healing on the next enumeration, you don't need to clean up by hand. The only failure mode is PID reuse: if process 84213 crashes and the OS later assigns 84213 to an unrelated process, the liveness check returns true and the file is kept. In practice this is rare on macOS (PIDs are assigned roughly monotonically up to a large cap) and harmless even when it happens — the file points at a dead MCP port, the agent fails to connect, retries the next one.

**`active.json` migrates on quit.** When the active instance exits gracefully, `shutdown()` deletes its instance file, deletes `active.json`, finds the next-most-recently-started live instance (by `started_at`), and writes that one as the new active. So legacy single-instance agents holding `active.json` get a fresh pointer. If the active crashes hard, this migration doesn't happen — but the next process that calls `read()` (via `instance.list` or otherwise) sees that `active.json` points to a dead PID, falls through to "scan instances/, pick most recent live one," and effectively does the same thing. So either way, single-instance agents end up at a live instance. The only window of inconsistency is between the crash and the next read.

**Focus events don't update the pointer.** Switching foreground windows by clicking, alt-tabbing, or anything else does not rewrite `active.json`. This is intentional — the user explicitly accepted that "focused window may not match `active.json` for short windows of time." The pointer's contract is "the most recently *launched* surviving instance," not "the focused instance." If your agent needs to know which window the user is currently looking at, that's an OS-level question (NSWorkspace / X11 EWMH `_NET_ACTIVE_WINDOW` / Win32 GetForegroundWindow) — not something the multi-instance protocol surfaces.

**Two instances can briefly show the same name.** The claim is atomic via O_EXCL `create_new`, so only one process ever owns `alpha.json`. But there's a window during startup — between O_EXCL claim and `write_initial` finishing — where the file exists but contains only `{}`. During that window an enumerator sees an entry with empty `id` and empty `pid` and skips it. Fine in practice; a mention here in case a debugger sees `{}` and panics.

**Port 19876 isn't sacred.** If alpha owns 19876 and bravo launches, bravo's MCP server tries 19876 (binds fail, alpha owns it), then 19877, 19878, … out to 19881, then OS-assigned. Don't hardcode `19876` anywhere — read `mcp_port` from the instance file. Same for `19877` on the HTTP side.

## Building this into your own agent

Forty lines of Python that opens a connection, picks an instance by cwd, lists its panes, sends text, and exits. No external dependencies beyond the standard library.

```python
import json, os, socket
from pathlib import Path

INSTANCES = Path.home() / ".unterm" / "instances"

def list_instances():
    out = []
    for f in INSTANCES.glob("*.json"):
        try:
            data = json.loads(f.read_text())
            if data.get("id") and data.get("pid"):
                out.append(data)
        except Exception:
            continue
    return out

class Unterm:
    def __init__(self, port, token):
        self.sock = socket.create_connection(("127.0.0.1", port))
        self.f = self.sock.makefile("rwb", buffering=0)
        self._next_id = 1
        self._call("auth.login", {"token": token})

    def _call(self, method, params=None):
        rid = self._next_id; self._next_id += 1
        req = {"jsonrpc": "2.0", "id": rid, "method": method, "params": params or {}}
        self.f.write((json.dumps(req) + "\n").encode())
        resp = json.loads(self.f.readline())
        if "error" in resp: raise RuntimeError(resp["error"]["message"])
        return resp["result"]

    def list(self):       return self._call("session.list")["sessions"]
    def send(self, i, t): return self._call("session.send_text", {"id": i, "text": t})
    def info(self):       return self._call("instance.info")
    def close(self):      self.sock.close()

# Pick the Unterm whose cwd ends with "/unterm" — fall back to most recent.
all_inst = list_instances()
target = next((i for i in all_inst if (i.get("cwd") or "").endswith("/unterm")), None)
if target is None:
    target = max(all_inst, key=lambda i: i["started_at"])

ut = Unterm(target["mcp_port"], target["auth_token"])
print("connected to", ut.info()["id"])
for s in ut.list():
    print(f"  pane {s['id']}: {s['title']} ({s['cols']}x{s['rows']})")
ut.send(0, "echo hello from python\n")
ut.close()
```

That's the whole client. Drop it into a script, point it at your agent, and you have multi-instance routing. The same skeleton works for any language with a TCP socket and a JSON parser.

The reference implementation lives at `wezterm-gui/src/server_info.rs` (storage layer) and `wezterm-gui/src/mcp/handler.rs` (the four `instance.*` methods). The `unterm-cli` binary uses the same protocol and is a useful smoke test: `unterm-cli instance list --json` does the discovery sequence and dumps it. If your agent's seeing different output than the CLI, the bug's in your client, not the server.

---

Source for everything described here lives at [github.com/unzooai/unterm](https://github.com/unzooai/unterm) under MIT. File issues / PRs there.
