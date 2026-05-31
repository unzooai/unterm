# Smithery submission

Smithery auto-discovers from `smithery.yaml` at the repo root, which is
already committed (see <https://github.com/unzooai/unterm/blob/master/smithery.yaml>).
You just need to claim the listing.

## Steps

1. Go to <https://smithery.ai>.
2. Click **Sign in** → **Continue with GitHub**.
3. Click **Add Server** (or **Submit a Server**).
4. Paste the GitHub URL: `https://github.com/unzooai/unterm`.
5. Smithery reads `smithery.yaml` and pre-fills name / description / install
   command. Confirm the prerequisite text (the "this needs the Unterm
   desktop app installed first" callout) is visible — that's the most
   important field.
6. Categories (pick up to 3): **Developer Tools**, **Terminal**, **Agent Orchestration**.
7. Submit.

Smithery's review usually takes hours. They'll list it under your
GitHub account; you can later transfer ownership to an `unzooai` org page
if you set one up on Smithery.

## What to verify after listing goes live

- The install snippet they show users matches `marketplace/mcp/configs/claude-desktop.json`.
- The "Install via Smithery CLI" button — Smithery offers a
  `npx -y @smithery/cli install unterm` flow. Make sure clicking it surfaces
  the prerequisite ("install the Unterm app from unterm.app first")
  rather than just running `npx`. If it doesn't, file a polish issue on
  their repo so they read the `prerequisites` block from our yaml.

## Backup: if smithery.yaml auto-discovery fails

Paste the contents of `marketplace/mcp/manifest.json` directly into
Smithery's "Manual configuration" tab. It's their long-form schema and
covers the same fields with more detail.
