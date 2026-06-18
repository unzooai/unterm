//! Static MCP + CLI surface tables, shared between the GUI (which serves
//! `meta.surface` and dispatches from `MCP_METHODS`) and `unterm-cli
//! mcp-stdio` (which falls back to these tables to answer `tools/list`
//! introspection when the GUI isn't running — e.g. registry health checks
//! or an agent starting before the terminal does).

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct McpMethod {
    pub name: &'static str,
    pub namespace: &'static str,
    pub summary: &'static str,
    pub params: &'static [Param],
}

#[derive(Debug, Clone, Serialize)]
pub struct Param {
    pub name: &'static str,
    pub kind: &'static str,
    pub required: bool,
    pub summary: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct CliCommand {
    pub name: &'static str,
    pub summary: &'static str,
    pub subcommands: &'static [&'static str],
}

const P_PANE_ID: Param = Param {
    name: "pane_id",
    kind: "string|int",
    required: false,
    summary: "Target pane; defaults to active pane where applicable.",
};
const P_SESSION_ID: Param = Param {
    name: "session_id",
    kind: "string",
    required: false,
    summary: "Alias for pane_id.",
};
const NO_PARAMS: &[Param] = &[];

// Single source of truth for MCP methods. `handler.rs` dispatch reads from
// this list to assert each name has a match arm; introspection callers
// read it for surface discovery. Adding a new method: write the match
// arm AND add an entry here, otherwise the dispatch self-check trips in
// debug builds.
pub const MCP_METHODS: &[McpMethod] = &[
    // ---- meta ----
    McpMethod {
        name: "meta.surface",
        namespace: "meta",
        summary: "Inventory of MCP methods + CLI subcommands + keybindings.",
        params: NO_PARAMS,
    },
    // ---- session ----
    McpMethod {
        name: "session.list",
        namespace: "session",
        summary: "List every live pane with its title, cwd, and shell.",
        params: NO_PARAMS,
    },
    McpMethod {
        name: "session.create",
        namespace: "session",
        summary: "Spawn a new tab; optionally specify cwd, command, or profile.",
        params: &[
            Param { name: "cwd", kind: "string", required: false, summary: "Working directory." },
            Param { name: "command", kind: "string", required: false, summary: "Override shell command." },
            Param { name: "profile", kind: "string", required: false, summary: "Identity profile name." },
        ],
    },
    McpMethod {
        name: "session.status",
        namespace: "session",
        summary: "Get pane state: title, cwd, shell, dims, busy flag.",
        params: &[P_PANE_ID, P_SESSION_ID],
    },
    McpMethod {
        name: "session.get",
        namespace: "session",
        summary: "Alias for session.status.",
        params: &[P_PANE_ID, P_SESSION_ID],
    },
    McpMethod {
        name: "session.split",
        namespace: "session",
        summary: "Split the current pane left/right/up/down.",
        params: &[
            P_PANE_ID,
            Param { name: "direction", kind: "string", required: true, summary: "left|right|up|down" },
        ],
    },
    McpMethod {
        name: "session.focus",
        namespace: "session",
        summary: "Bring a pane into focus.",
        params: &[P_PANE_ID, P_SESSION_ID],
    },
    McpMethod {
        name: "session.input",
        namespace: "session",
        summary: "Send a keystroke or text into the pane (audited; subject to confirmation).",
        params: &[
            P_PANE_ID,
            Param { name: "text", kind: "string", required: true, summary: "Raw bytes to inject." },
        ],
    },
    McpMethod {
        name: "session.resize",
        namespace: "session",
        summary: "Resize a pane to the given cols/rows.",
        params: &[P_PANE_ID, Param { name: "cols", kind: "int", required: true, summary: "" }, Param { name: "rows", kind: "int", required: true, summary: "" }],
    },
    McpMethod {
        name: "session.destroy",
        namespace: "session",
        summary: "Close a pane.",
        params: &[P_PANE_ID, P_SESSION_ID],
    },
    McpMethod {
        name: "session.idle",
        namespace: "session",
        summary: "Idle/activity stats for a pane.",
        params: &[P_PANE_ID, P_SESSION_ID],
    },
    McpMethod {
        name: "session.cwd",
        namespace: "session",
        summary: "Read a pane's current working directory.",
        params: &[P_PANE_ID, P_SESSION_ID],
    },
    McpMethod {
        name: "session.env",
        namespace: "session",
        summary: "Read pane environment status; returns an unsupported marker in WezTerm mode.",
        params: &[P_PANE_ID, P_SESSION_ID],
    },
    McpMethod {
        name: "session.set_env",
        namespace: "session",
        summary: "Request pane environment mutation; returns an unsupported marker in WezTerm mode.",
        params: &[
            P_PANE_ID,
            P_SESSION_ID,
            Param { name: "name", kind: "string", required: false, summary: "Environment variable name." },
            Param { name: "value", kind: "string", required: false, summary: "Environment variable value." },
        ],
    },
    McpMethod {
        name: "session.history",
        namespace: "session",
        summary: "Shell history for a pane (when shell integration is on).",
        params: &[P_PANE_ID, P_SESSION_ID],
    },
    McpMethod {
        name: "session.audit_log",
        namespace: "session",
        summary: "Last N audited MCP/CLI write actions on this instance.",
        params: NO_PARAMS,
    },
    McpMethod {
        name: "session.suggest",
        namespace: "session",
        summary: "Propose text without touching the PTY — user accepts with Tab.",
        params: &[P_PANE_ID, Param { name: "text", kind: "string", required: true, summary: "" }, Param { name: "ttl_ms", kind: "int", required: false, summary: "" }],
    },
    McpMethod {
        name: "session.suggest_status",
        namespace: "session",
        summary: "Lifecycle state of a pending suggest.",
        params: &[Param { name: "id", kind: "string", required: true, summary: "" }],
    },
    McpMethod {
        name: "session.suggest_cancel",
        namespace: "session",
        summary: "Withdraw a pending suggest.",
        params: &[Param { name: "id", kind: "string", required: true, summary: "" }],
    },
    McpMethod {
        name: "session.suggest_list",
        namespace: "session",
        summary: "List active suggests across all panes.",
        params: NO_PARAMS,
    },
    McpMethod {
        name: "session.recording_start",
        namespace: "session",
        summary: "Begin recording the active pane as an asciicast.",
        params: &[P_PANE_ID],
    },
    McpMethod {
        name: "session.recording_stop",
        namespace: "session",
        summary: "Stop recording.",
        params: &[P_PANE_ID],
    },
    McpMethod {
        name: "session.recording_status",
        namespace: "session",
        summary: "Recording state for a pane.",
        params: &[P_PANE_ID],
    },
    McpMethod {
        name: "session.recording_list",
        namespace: "session",
        summary: "List recorded sessions in <project>/.unterm/sessions/.",
        params: NO_PARAMS,
    },
    McpMethod {
        name: "session.recording_read",
        namespace: "session",
        summary: "Read a recorded session's rendered Markdown.",
        params: &[Param { name: "session_id", kind: "string", required: true, summary: "Recording/session id." }],
    },
    McpMethod {
        name: "session.recording_attach_trace",
        namespace: "session",
        summary: "Attach an external trace id to the active recording for later correlation.",
        params: &[P_PANE_ID, Param { name: "trace_id", kind: "string", required: true, summary: "Trace id to associate with the recording." }],
    },
    McpMethod {
        name: "session.export_markdown",
        namespace: "session",
        summary: "Export the active pane scrollback as Markdown.",
        params: &[P_PANE_ID, Param { name: "path", kind: "string", required: false, summary: "Optional destination file path." }],
    },
    // ---- exec ----
    McpMethod {
        name: "exec.run",
        namespace: "exec",
        summary: "Fire-and-forget command in a pane (returns immediately).",
        params: &[P_PANE_ID, Param { name: "command", kind: "string", required: true, summary: "" }],
    },
    McpMethod {
        name: "exec.send",
        namespace: "exec",
        summary: "Send raw bytes to a pane's PTY (audited).",
        params: &[P_PANE_ID, Param { name: "bytes", kind: "string", required: true, summary: "" }],
    },
    McpMethod {
        name: "exec.run_wait",
        namespace: "exec",
        summary: "Run a command and block until it exits, returning output.",
        params: &[P_PANE_ID, Param { name: "command", kind: "string", required: true, summary: "" }, Param { name: "timeout_ms", kind: "int", required: false, summary: "" }],
    },
    McpMethod {
        name: "exec.status",
        namespace: "exec",
        summary: "Poll a running exec.run command.",
        params: &[Param { name: "id", kind: "string", required: true, summary: "Exec id." }],
    },
    McpMethod {
        name: "exec.cancel",
        namespace: "exec",
        summary: "Cancel a running exec.run command.",
        params: &[Param { name: "id", kind: "string", required: true, summary: "" }],
    },
    McpMethod {
        name: "signal.send",
        namespace: "exec",
        summary: "Deliver a signal to a pane's foreground process.",
        params: &[P_PANE_ID, Param { name: "signal", kind: "string", required: true, summary: "INT|TERM|KILL|HUP|USR1|USR2" }],
    },
    // ---- screen ----
    McpMethod {
        name: "screen.read",
        namespace: "screen",
        summary: "Read the visible viewport as a cell grid (rows of cells with style).",
        params: &[P_PANE_ID, P_SESSION_ID],
    },
    McpMethod {
        name: "screen.text",
        namespace: "screen",
        summary: "Read the visible viewport as plain text lines.",
        params: &[P_PANE_ID, P_SESSION_ID],
    },
    McpMethod {
        name: "screen.scrollback_text",
        namespace: "screen",
        summary: "Dump the entire scrollback + viewport as text (LLM hand-off).",
        params: &[
            P_PANE_ID,
            P_SESSION_ID,
            Param { name: "escapes", kind: "bool", required: false, summary: "Preserve ANSI escapes." },
            Param { name: "start_line", kind: "int", required: false, summary: "Absolute StableRowIndex." },
            Param { name: "end_line", kind: "int", required: false, summary: "Absolute StableRowIndex (exclusive)." },
        ],
    },
    McpMethod {
        name: "screen.cursor",
        namespace: "screen",
        summary: "Read the cursor position + shape.",
        params: &[P_PANE_ID, P_SESSION_ID],
    },
    McpMethod {
        name: "screen.scroll",
        namespace: "screen",
        summary: "Scroll the pane's viewport.",
        params: &[P_PANE_ID, Param { name: "lines", kind: "int", required: true, summary: "Positive scrolls down; negative scrolls up." }],
    },
    McpMethod {
        name: "screen.search",
        namespace: "screen",
        summary: "Find a substring in the scrollback; optionally jump the viewport to a match.",
        params: &[
            P_PANE_ID,
            Param { name: "pattern", kind: "string", required: true, summary: "" },
            Param { name: "goto", kind: "bool", required: false, summary: "Scroll the GUI viewport to the first match." },
            Param { name: "goto_match", kind: "int", required: false, summary: "Jump to the Nth match (0-based; implies goto)." },
        ],
    },
    McpMethod {
        name: "screen.detect_errors",
        namespace: "screen",
        summary: "Heuristic scan for error-shaped lines in recent output.",
        params: &[P_PANE_ID, P_SESSION_ID],
    },
    // ---- ghost ----
    McpMethod {
        name: "ghost.debug",
        namespace: "ghost",
        summary: "Read ghost-text predictor/debug state for a pane.",
        params: &[P_PANE_ID],
    },
    // ---- orchestrate ----
    McpMethod {
        name: "orchestrate.launch",
        namespace: "orchestrate",
        summary: "Create a new tab and optionally run a command in it.",
        params: &[
            Param { name: "cwd", kind: "string", required: false, summary: "Working directory." },
            Param { name: "command", kind: "string", required: false, summary: "Command to run after the shell starts." },
            Param { name: "profile", kind: "string", required: false, summary: "Identity profile name." },
        ],
    },
    McpMethod {
        name: "orchestrate.broadcast",
        namespace: "orchestrate",
        summary: "Send the same command to multiple panes.",
        params: &[
            Param { name: "command", kind: "string", required: true, summary: "Command to send." },
            Param { name: "sessions", kind: "array", required: true, summary: "Pane/session ids to receive the command." },
        ],
    },
    McpMethod {
        name: "orchestrate.wait",
        namespace: "orchestrate",
        summary: "Wait until a pane's visible text contains a pattern.",
        params: &[
            P_PANE_ID,
            Param { name: "pattern", kind: "string", required: true, summary: "Text to wait for." },
            Param { name: "timeout_ms", kind: "int", required: false, summary: "Maximum wait time." },
        ],
    },
    // ---- workspace ----
    McpMethod {
        name: "workspace.save",
        namespace: "workspace",
        summary: "Persist current window/tab/pane layout as a named workspace.",
        params: &[Param { name: "name", kind: "string", required: true, summary: "" }],
    },
    McpMethod {
        name: "workspace.restore",
        namespace: "workspace",
        summary: "Re-create panes from a saved workspace.",
        params: &[Param { name: "name", kind: "string", required: true, summary: "" }],
    },
    McpMethod {
        name: "workspace.list",
        namespace: "workspace",
        summary: "Names of saved workspaces.",
        params: NO_PARAMS,
    },
    // ---- capture ----
    McpMethod {
        name: "capture.screen",
        namespace: "capture",
        summary: "PNG of the whole screen (multi-monitor aware).",
        params: NO_PARAMS,
    },
    McpMethod {
        name: "capture.window",
        namespace: "capture",
        summary: "PNG of a specific window matched by title/pid.",
        params: &[
            Param { name: "title", kind: "string", required: false, summary: "" },
            Param { name: "pid", kind: "int", required: false, summary: "" },
            Param { name: "include_base64", kind: "bool", required: false, summary: "" },
        ],
    },
    McpMethod {
        name: "capture.select",
        namespace: "capture",
        summary: "Interactive region select (headless falls back to screen).",
        params: NO_PARAMS,
    },
    McpMethod {
        name: "capture.clipboard",
        namespace: "capture",
        summary: "Snapshot the user's clipboard (text or image).",
        params: NO_PARAMS,
    },
    McpMethod {
        name: "capture.scrollback",
        namespace: "capture",
        summary: "Scrolling screenshot of a pane: render the ENTIRE scrollback to one tall PNG (headless re-render, works even occluded). Prefer screen.scrollback_text when only the text matters.",
        params: &[
            Param { name: "id", kind: "int", required: false, summary: "Pane id (default: active pane)." },
            Param { name: "max_rows", kind: "int", required: false, summary: "Row cap, keeps the most recent rows (default 10000)." },
            Param { name: "dpi", kind: "int", required: false, summary: "Raster dpi 48-288 (default 144 on macOS, 96 elsewhere)." },
        ],
    },
    McpMethod {
        name: "capture.window_scroll",
        namespace: "capture",
        summary: "Scrolling (long) screenshot of ANOTHER app's window: synthesize wheel events + stitch frames (macOS).",
        params: &[
            Param { name: "app", kind: "string", required: false, summary: "App name substring, e.g. 'Safari'." },
            Param { name: "title", kind: "string", required: false, summary: "Window title substring." },
            Param { name: "pid", kind: "int", required: false, summary: "Owning process id." },
            Param { name: "under_cursor", kind: "bool", required: false, summary: "Target the window under the mouse pointer." },
            Param { name: "max_frames", kind: "int", required: false, summary: "Frame cap (default 25)." },
            Param { name: "activate", kind: "bool", required: false, summary: "Raise the target window first (default true)." },
            Param { name: "restore_scroll", kind: "bool", required: false, summary: "Scroll back up afterwards (default true)." },
        ],
    },
    // ---- upload ----
    McpMethod {
        name: "upload.file",
        namespace: "upload",
        summary: "PUT a local file to user-configured OSS/COS/Qiniu; return public URL.",
        params: &[
            Param { name: "path", kind: "string", required: true, summary: "Absolute local path." },
            Param { name: "provider", kind: "string", required: false, summary: "oss|cos|qiniu (default from config)." },
            Param { name: "key", kind: "string", required: false, summary: "Destination object key." },
        ],
    },
    // ---- proxy ----
    McpMethod { name: "proxy.status", namespace: "proxy", summary: "Current OS proxy + Unterm overrides.", params: NO_PARAMS },
    McpMethod { name: "proxy.nodes", namespace: "proxy", summary: "Known proxy nodes from the user's config.", params: NO_PARAMS },
    McpMethod { name: "proxy.switch", namespace: "proxy", summary: "Switch to a named proxy node.", params: &[Param { name: "node", kind: "string", required: true, summary: "" }] },
    McpMethod { name: "proxy.speedtest", namespace: "proxy", summary: "Latency test against known proxy endpoints.", params: NO_PARAMS },
    McpMethod { name: "proxy.configure", namespace: "proxy", summary: "Apply proxy settings to the OS (macOS scutil / Windows reg / Linux env).", params: &[Param { name: "url", kind: "string", required: true, summary: "" }] },
    McpMethod { name: "proxy.disable", namespace: "proxy", summary: "Clear all OS proxy overrides.", params: NO_PARAMS },
    McpMethod { name: "proxy.env", namespace: "proxy", summary: "Effective proxy-related env vars.", params: NO_PARAMS },
    McpMethod { name: "proxy.rotation", namespace: "proxy", summary: "Get/set endpoint-level auto-rotation: fail over to the fastest live node in a pool when the active one dies.", params: &[Param { name: "enabled", kind: "bool", required: false, summary: "Turn auto-rotation on/off." }, Param { name: "pool", kind: "string", required: false, summary: "List of node names eligible for rotation." }, Param { name: "interval_secs", kind: "int", required: false, summary: "Health-check cadence (min 10)." }] },
    McpMethod { name: "proxy.set_nodes", namespace: "proxy", summary: "Replace the proxy node list (name+url pairs) for the rotation pool, without hand-editing proxy.json.", params: &[Param { name: "nodes", kind: "array", required: true, summary: "Array of {name, url} objects." }] },
    McpMethod { name: "proxy.clash_status", namespace: "proxy", summary: "Read the Clash/mihomo controller: switchable groups + their nodes with live alive/delay. Powers click-to-build rotation pools.", params: NO_PARAMS },
    McpMethod { name: "proxy.clash_select", namespace: "proxy", summary: "Point a Clash Selector group at a node via the controller API.", params: &[Param { name: "group", kind: "string", required: true, summary: "Selector group name." }, Param { name: "name", kind: "string", required: true, summary: "Node to switch to." }] },
    McpMethod { name: "proxy.clash_set_controller", namespace: "proxy", summary: "Set/clear a manual Clash controller (host:port + secret) for when auto-discovery can't find it (e.g. Windows).", params: &[Param { name: "controller", kind: "string", required: false, summary: "host:port, empty to clear." }, Param { name: "secret", kind: "string", required: false, summary: "Bearer secret if required." }] },
    // ---- governance ----
    McpMethod { name: "policy.set", namespace: "governance", summary: "Update MCP write-confirmation policy.", params: NO_PARAMS },
    McpMethod { name: "policy.check", namespace: "governance", summary: "Test whether a command would be allowed by policy.", params: &[Param { name: "command", kind: "string", required: true, summary: "" }] },
    McpMethod { name: "server.info", namespace: "governance", summary: "Server version, uptime, instance id.", params: NO_PARAMS },
    McpMethod { name: "server.health", namespace: "governance", summary: "Liveness + readiness flags.", params: NO_PARAMS },
    McpMethod { name: "server.capabilities", namespace: "governance", summary: "Method namespace map (back-compat — prefer meta.surface).", params: NO_PARAMS },
    McpMethod { name: "selftest.run", namespace: "governance", summary: "Run the built-in MCP self-test suite.", params: NO_PARAMS },
    McpMethod { name: "agent.identify", namespace: "governance", summary: "Self-tag the calling agent for audit grouping.", params: &[Param { name: "name", kind: "string", required: true, summary: "" }] },
    McpMethod { name: "agent.whoami", namespace: "governance", summary: "Read the calling agent's self-tag.", params: NO_PARAMS },
    McpMethod { name: "agent.list_trusted", namespace: "governance", summary: "List runtime, configured, and persisted trusted agent names.", params: NO_PARAMS },
    McpMethod { name: "agent.trust", namespace: "governance", summary: "Trust an agent name so future PTY writes skip confirmation.", params: &[Param { name: "agent", kind: "string", required: true, summary: "Agent name to trust." }] },
    McpMethod { name: "agent.untrust", namespace: "governance", summary: "Remove an agent name from the persistent trust list.", params: &[Param { name: "agent", kind: "string", required: true, summary: "Agent name to revoke." }] },
    McpMethod { name: "profile.list", namespace: "governance", summary: "List identity profiles without exposing secret values.", params: NO_PARAMS },
    McpMethod { name: "profile.current", namespace: "governance", summary: "Read the identity profile bound to this Unterm instance.", params: NO_PARAMS },
    McpMethod { name: "profile.audit", namespace: "governance", summary: "Report expiring profile secrets without revealing secret values.", params: NO_PARAMS },
    // ---- system ----
    McpMethod { name: "system.info", namespace: "system", summary: "OS, arch, hostname, locale.", params: NO_PARAMS },
    McpMethod { name: "system.launch_admin", namespace: "system", summary: "Re-launch Unterm with elevated privileges (UAC/sudo prompt).", params: NO_PARAMS },
    // ---- instance ----
    McpMethod { name: "instance.list", namespace: "instance", summary: "Enumerate live Unterm instances on this machine.", params: NO_PARAMS },
    McpMethod { name: "instance.info", namespace: "instance", summary: "Details for one instance (this one by default).", params: &[Param { name: "id", kind: "string", required: false, summary: "alpha|bravo|charlie…" }] },
    McpMethod { name: "instance.set_title", namespace: "instance", summary: "Override the instance window title.", params: &[Param { name: "title", kind: "string", required: true, summary: "" }] },
    McpMethod { name: "instance.focus", namespace: "instance", summary: "Bring this instance's main window to the front.", params: NO_PARAMS },
];

// CLI inventory. The MCP server runs in a different binary than the CLI,
// so we can't introspect clap directly — keep this list in sync with
// `wezterm/src/main.rs`'s `SubCommand` enum. The `unterm-cli reference`
// command round-trips through this, so a mismatched entry is visible
// in the self-test.
pub const CLI_COMMANDS: &[CliCommand] = &[
    CliCommand { name: "start", summary: "Start the GUI, optionally running an alternative program.", subcommands: &[] },
    CliCommand { name: "cli", summary: "Interact with the mux server (panes, tabs, windows).", subcommands: &["list", "list-clients", "proxy", "tlscreds", "move-pane-to-new-tab", "split-pane", "spawn", "send-text", "get-text", "activate-pane-direction", "get-pane-direction", "kill-pane", "activate-pane", "adjust-pane-size", "activate-tab", "set-tab-title", "set-window-title", "rename-workspace", "zoom-pane"] },
    CliCommand { name: "session", summary: "Operate on a single live pane.", subcommands: &["list", "create", "record", "export"] },
    CliCommand { name: "sessions", summary: "Browse the recorded session archive.", subcommands: &["list", "read"] },
    CliCommand { name: "workspace", summary: "Save or restore named pane workspaces.", subcommands: &["list", "save", "restore"] },
    CliCommand { name: "instance", summary: "List, inspect, label, or focus live Unterm instances.", subcommands: &["list", "info", "set-title", "focus"] },
    CliCommand { name: "screenshot", summary: "Capture the screen via Unterm's MCP server. --scrollback = long screenshot of a pane's entire history; --scroll-app/--scroll-title = scroll + stitch another app's window (macOS).", subcommands: &[] },
    CliCommand { name: "upload", summary: "Upload a local file to your configured object storage and print the public URL.", subcommands: &["config-path"] },
    CliCommand { name: "scrollback", summary: "Dump the full scrollback + viewport of a pane as text.", subcommands: &[] },
    CliCommand { name: "reference", summary: "Print MCP methods, CLI subcommands, and live keybindings.", subcommands: &[] },
    CliCommand { name: "setup-ai", summary: "Register or unregister Unterm with local AI coding agents.", subcommands: &[] },
    CliCommand { name: "mcp-stdio", summary: "Run an MCP stdio bridge so an AI agent can drive this instance.", subcommands: &[] },
    CliCommand { name: "settings", summary: "Open the Unterm Web Settings UI in your browser.", subcommands: &["open"] },
    CliCommand { name: "proxy", summary: "Manage Unterm's proxy via the MCP server.", subcommands: &["status", "nodes", "switch", "disable", "env", "rotation"] },
    CliCommand { name: "theme", summary: "List / switch Unterm theme presets.", subcommands: &["list", "switch"] },
    CliCommand { name: "profile", summary: "Manage identity profiles (GitHub / AWS / npm tokens, git identity, SSH keys).", subcommands: &["list", "create", "show", "set-secret", "delete", "audit", "edit", "export", "spawn", "import", "set-default", "shell-integration"] },
    CliCommand { name: "agent", summary: "Install, authenticate, configure, and launch AI coding-agent CLIs.", subcommands: &["list", "show", "install", "update", "uninstall", "auth", "configure", "import", "plan", "launch", "run", "manifest"] },
    CliCommand { name: "lang", summary: "List, set, or print the active interface locale.", subcommands: &["list", "set", "current"] },
    CliCommand { name: "show-keys", summary: "Show key assignments (effective from config).", subcommands: &[] },
    CliCommand { name: "ls-fonts", summary: "Display information about fonts.", subcommands: &[] },
    CliCommand { name: "imgcat", summary: "Output an image to the terminal.", subcommands: &[] },
    CliCommand { name: "set-working-directory", summary: "Emit an OSC 7 escape so the terminal learns the cwd.", subcommands: &[] },
    CliCommand { name: "record", summary: "Record a terminal session as an asciicast.", subcommands: &[] },
    CliCommand { name: "replay", summary: "Replay an asciicast terminal session.", subcommands: &[] },
    CliCommand { name: "ssh", summary: "Establish an SSH session.", subcommands: &[] },
    CliCommand { name: "connect", summary: "Connect to a Unterm mux server.", subcommands: &[] },
    CliCommand { name: "shell-completion", summary: "Generate shell completion information.", subcommands: &[] },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_surface_includes_agent_onboarding_entrypoints() {
        let names: std::collections::HashSet<_> = CLI_COMMANDS.iter().map(|c| c.name).collect();
        for required in ["reference", "setup-ai", "mcp-stdio", "agent", "settings"] {
            assert!(
                names.contains(required),
                "CLI_COMMANDS is missing {required}"
            );
        }
    }

    #[test]
    fn mcp_surface_includes_agent_and_profile_discovery() {
        let names: std::collections::HashSet<_> = MCP_METHODS.iter().map(|m| m.name).collect();
        for required in [
            "meta.surface",
            "agent.identify",
            "agent.list_trusted",
            "agent.trust",
            "agent.untrust",
            "profile.list",
            "profile.current",
            "profile.audit",
            "orchestrate.launch",
            "orchestrate.broadcast",
            "orchestrate.wait",
            "session.recording_attach_trace",
        ] {
            assert!(
                names.contains(required),
                "MCP_METHODS is missing {required}"
            );
        }
    }
}
