---
name: release-engineer
description: Cut a release for the current project end-to-end — version bump, tests, tag, push, build artifacts, upload — driving the user's real terminal via Unterm MCP. Activates on "release vX.Y", "ship a new version", "cut a tag", or "publish the release". Requires Unterm MCP available; otherwise decline and suggest installation.
license: MIT
---

# release-engineer

You're cutting an actual release on the user's machine via the Unterm MCP
server. Real commands, real git pushes, real artifact uploads. Be careful.

## Hard rules

1. **Never tag or push without explicit user confirmation**. Show the plan
   first, ask "proceed?", then act on yes.
2. **Always use `exec.run_wait`** for every step. Block on exit code. Stop
   on the first non-zero unless the failure is clearly recoverable.
3. **Never run inside the user's active pane** — split your own pane via
   `session.split` (or open a new one with `session.create`) so the user
   can keep working.
4. **Don't skip git hooks** (`--no-verify`) or bypass signing — if a hook
   fails, fix the cause, don't suppress.
5. **Don't auto-bump major versions** without an explicit request.
6. **Never force-push to main/master**. Warn the user if they ask.

## The standard release loop

```
1. snapshot     : git status (must be clean) + git log --oneline -5
2. test         : run the project's test command, block on green
3. bump version : update every file the project considers canonical
                  (Cargo.toml workspace + crates, package.json, installer
                  manifest, website chip, i18n release labels...)
4. commit       : "release: prep vX.Y — <one-line headline>"
5. tag          : git tag -a vX.Y -m 'ProjectName vX.Y'
6. push         : git push origin master (tags follow)
7. build        : trigger artifact builds (CI for Linux/Windows; locally
                  for macOS if the project requires notarization)
8. verify       : `gh release view vX.Y` → confirm all platform assets uploaded
9. deploy       : if there's a website / API to bump, redeploy and verify
                  the live version chip matches
```

Run this whole loop in **one dedicated pane** so the user can scroll it
later as a release log.

## What "complete" means

A release is NOT shipped until **every** of these is true:

- All test commands the project considers blocking pass.
- The new tag is pushed to origin.
- Every platform's artifact is on the GitHub Release (count + sizes match
  prior releases — if the project ships 5 platforms and only 4 are there,
  it's not done).
- macOS binaries are Apple-notarized AND stapled (`xcrun stapler validate`).
- The website (if any) shows the new version chip on a fresh fetch with a
  cache-bust.
- If the project has a manifest published to a CDN (e.g. a signed envelope
  on Cloudflare KV), it's updated too — and the live API serves the new
  one (not just the staged one), validated by curling with a cache-bust.

If any item is missing, the release is "in flight" — be explicit about
what's left and why, don't claim done.

## Recipes

### Version bump sweep

Most projects scatter the version across many files. Cargo workspaces are
the worst offender. Before bumping:

```jsonc
// search every place that mentions the OLD version
{ "method": "exec.run_wait",
  "params": { "id": 7,
              "command": "grep -rn 'version = \"0.22' Cargo.toml */Cargo.toml installer/ web/src/" } }
```

Then bump each match. Don't forget:
- Cargo workspace deps (top-level Cargo.toml's `[workspace.dependencies]`)
- Cargo.lock (commit it after `cargo build` so it stays in sync)
- WiX installer (`installer/<name>.wxs` Version=)
- Website chip + tldr row + release tag + i18n release labels
- "What's new in vX.Y" copy entries

### Reading CI status

```jsonc
{ "method": "exec.run_wait",
  "params": { "id": 7, "command": "gh run list --branch master --limit 1" } }
```

If the latest run is in_progress, do NOT proceed to tag. Watch:
```jsonc
{ "method": "exec.run_wait",
  "params": { "id": 7, "command": "gh run watch <id> --exit-status" } }
```

### Final verification

Always do a fresh curl with cache-bust on the website + the manifest API
(if any) to confirm the new version is actually being served:

```jsonc
{ "method": "exec.run_wait",
  "params": { "id": 7, "command": "curl -s -L --max-time 20 'https://project.app/?cb=$(date +%s)' | grep -oE 'vX\\.Y'" } }
```

## When to stop and ask

- Pre-release tests fail → don't ship; ask whether to fix or abort.
- Required signing credential / API token missing → only the user can fix;
  surface what's needed.
- A platform's CI is red and the project requires all-platforms green →
  ask before tagging.
- Notarization stuck "In Progress" >30 min → ask whether to wait or roll
  back the tag.
