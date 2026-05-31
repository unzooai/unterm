# Cursor MCP directory submission

Cursor's MCP directory is editorially curated — there's no public
submission form. The path is **a short, polite email** to their
partnerships address, or a message in the Cursor community Discord
(<https://discord.gg/cursor>).

## Email draft (paste-ready)

**To**: `mcp@cursor.com` (or `partnerships@cursor.com` if mcp@ bounces)
**Subject**: `Listing request: Unterm MCP server — drive a real terminal from Cursor`

```text
Hi Cursor team,

I'd like to request a listing in the Cursor MCP directory for
Unterm — a cross-platform terminal (macOS / Linux / Windows) that
exposes itself as an MCP server with 67 methods across 11 namespaces.

Why it's a good fit for Cursor users:
- Cursor agents can spawn shells, split panes, type into them, and
  read scrollback — without ever simulating keystrokes.
- The agent that runs your code lives next to the agent that wrote it.
- Local-first: 127.0.0.1 only, bearer-token gated, per-call audit log.
  Every session.input is recorded with the calling agent's identity;
  first write triggers an Allow / Block prompt.
- One-click install of other coding agents (Claude Code, Codex, Gemini,
  OpenCode, Aider) from inside Unterm with auto-MCP-wiring.

Install for Cursor users is one snippet (~/.cursor/mcp.json or
Settings → MCP → Add), prerequisite is the Unterm desktop app
(https://unterm.app/api/download — UA-aware redirect).

Links:
- Repo:       https://github.com/unzooai/unterm
- Homepage:   https://unterm.app
- Docs:       https://unterm.app/docs/mcp-reference
- Cursor-specific config: https://github.com/unzooai/unterm/blob/master/marketplace/mcp/configs/cursor.json
- Full marketplace kit:   https://github.com/unzooai/unterm/tree/master/marketplace

Happy to provide screenshots, a 60s screen recording, or a longer
write-up — let me know what format fits your directory.

Thanks!
Alex Lee
```

## Discord pitch (shorter, paste-ready)

Channel: `#mcp` or `#showcase` (whichever is more active when you check)

> Just shipped **Unterm** — a cross-platform terminal (mac / linux /
> win) that exposes itself as a 67-method MCP server. Lets Cursor
> agents drive a real terminal: spawn panes, split them, type into
> them, read scrollback. Local-first (127.0.0.1 + bearer token +
> per-call audit). MIT, $0.
>
> Cursor config snippet:
> https://github.com/unzooai/unterm/blob/master/marketplace/mcp/configs/cursor.json
>
> Would love a spot in the official MCP directory — pinging here
> per the community-submit norm. Repo / docs / screenshots:
> https://github.com/unzooai/unterm

Attach the OG card image (`https://unterm.app/assets/og.png`) to the
Discord post — Discord auto-renders it as a preview, much higher
click-through than a bare link.
