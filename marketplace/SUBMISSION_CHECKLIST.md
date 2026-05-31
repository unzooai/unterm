# Submission checklist

Where to post Unterm's MCP server (and the Skills bundles), in priority
order. Each row: the marketplace, the submission method, the materials
required from this kit, and a status box you can tick as you go.

Legend: ☐ not yet submitted · ⏳ submitted, awaiting review · ✅ live

## MCP server marketplaces

| Status | Marketplace | URL | Submission method | Required materials |
| :-: | --- | --- | --- | --- |
| ☐ | **Anthropic / MCP official registry** | <https://github.com/modelcontextprotocol/servers> | Open a PR adding `Unterm` to the README's "Community servers" table, link to <https://unterm.app> + this repo. They don't host binaries; they index. | `marketplace/README.md` quick-pitch + capability table. |
| ☐ | **Smithery.ai** | <https://smithery.ai/new> | Sign in with GitHub, point at this repo, paste `mcp/manifest.json` content (or let it auto-discover via `smithery.yaml` at repo root — TODO add that file). | `mcp/manifest.json`. |
| ☐ | **mcp.so** | <https://mcp.so/submit> | Form submission. Needs: name, tagline, repo, homepage, transports, install command. | Manifest + 1-line tagline + `mcp/configs/README.md`. |
| ☐ | **Glama.ai / mcp** | <https://glama.ai/mcp/servers> | They auto-index from GitHub topic `mcp-server` — add that topic to the repo. Manual submission also possible via their Discord. | GitHub topic `mcp-server` on `unzooai/unterm`. |
| ☐ | **mcpservers.org** | <https://github.com/wong2/awesome-mcp-servers> | PR to the `README.md` adding an entry under "Terminal" or "System Tools" category. | One-line description + repo link. |
| ☐ | **awesome-mcp-servers** (punkpeye) | <https://github.com/punkpeye/awesome-mcp-servers> | PR adding an entry under "🤖 AI" or "💻 Command Line" (probably the latter — Unterm is a terminal). | One-line + repo link + emoji platform tag. |
| ☐ | **Claude Desktop catalog** | Built-in: Settings → MCP → Browse | Auto-discovers from official Anthropic registry. Submit there (row 1) and Claude Desktop picks it up. | (same as row 1) |
| ☐ | **Cursor MCP directory** | <https://cursor.com/mcp> | Cursor curates manually; reach out via their Discord / `mcp@cursor.com` with the manifest URL. | `mcp/manifest.json` + `mcp/configs/cursor.json`. |
| ☐ | **OpenAI / ChatGPT MCP catalog** | <https://platform.openai.com/docs/mcp> (when public submission opens) | Currently invite-only / partner-only. Hold. | TBD when GA. |

### GitHub repo prep (one-time)

Before submitting anywhere, set on `github.com/unzooai/unterm`:

- **Topics**: `mcp`, `mcp-server`, `model-context-protocol`, `terminal`,
  `tty`, `claude`, `cursor`, `agent-tools`, `wezterm` (auto-picked by
  Glama and several other indexers).
- **Repo description** (right sidebar): the one-line tagline from
  `marketplace/README.md`.
- **Repo website link**: <https://unterm.app>.
- **Pinned releases**: keep the latest minor tagged release pinned.

## Skills directories

The Anthropic Skills format (`SKILL.md` + YAML frontmatter) is shared
via GitHub repos rather than a single registry. Two paths:

| Status | Where | How |
| :-: | --- | --- |
| ☐ | **In-repo `skills/` directory** (this kit) | Users `git clone unzooai/unterm` and copy `marketplace/skills/<name>/` into `~/.claude/skills/<name>/`. Document in main README. | already done — files are at `marketplace/skills/` |
| ☐ | **awesome-claude-skills (community list)** | <https://github.com/anthropics/anthropic-cookbook> (Skills section) — PR adding our three SKILL.md links | the three `SKILL.md` files + 1-line description each |
| ☐ | **Claude.ai shared skills** (when sharing opens GA) | Upload via Claude Settings → Skills → Share | each `SKILL.md` text |

The three Skills authored:

1. **`terminal-control`** — base recipe for any agent that wants to drive a
   terminal via Unterm MCP. The "Hello, World" skill.
2. **`release-engineer`** — opinionated end-to-end release workflow
   (version bump → tests → tag → push → notarize → upload → verify).
   Replicates what `release-mac.sh` codifies, in agent-driven form.
3. **`parallel-agents`** — fan a task out to 3+ inner agents in side-by-side
   panes (one Claude, one Codex, one Gemini), then reconcile.

## After-submission housekeeping

Each marketplace listing should:

- Link **Download** to <https://unterm.app/api/download> (UA-aware redirect
  to the right OS artifact — single canonical URL).
- Link **Docs** to <https://unterm.app/docs/mcp-reference> (live MCP method
  reference).
- Quote the **headline metric**: "67 methods across 11 namespaces".
- Mention the **no-config-needed launch path**: if the user installs
  Unterm and uses *Shell → AI Agents* to launch an inner agent, the MCP
  is auto-wired with no copy-paste.

## Tracking submissions

When you submit, edit this file: flip ☐ → ⏳ and add the PR / submission URL
in a footnote. When it lands, flip ⏳ → ✅. This file becomes the audit
trail.
