# Changelog

## v0.57.1 — 2026-07-22

### Fixed

- **macOS: Ctrl+Left-click is treated as a secondary click only when CTRL is the sole modifier.** The v0.57.0 pointing-device workaround matched any CTRL-containing chord, which hijacked the default Ctrl+Shift window-drag gesture and silently shadowed user CTRL mouse bindings — each of them pasting the clipboard instead. A consumed Ctrl+Left press now also swallows the rest of that physical gesture, so a single click can no longer both paste and trigger the default drag/release bindings (extend selection, open link).
- **An empty clipboard no longer shows a "couldn't paste" error.** Right-clicking with nothing on the clipboard is a normal no-op now, on every platform; the red failure notice is reserved for real read failures. (Live-verified on macOS along with copy, paste, and Ctrl+click secondary paste.)
- **Sidebar: the active tab is revealed again when row churn pushes it out of view.** Reordering the active tab or closing tabs above it scrolls it back into the viewport; scrolling away yourself still isn't overridden.
- **Sidebar: duplicate project names always get a parent hint.** When one project path's tail is contained in another's (`/acme/app` vs `/work/acme/app`), the shorter one now falls back to its immediate parent instead of rendering an ambiguous bare name; narrow headers no longer overflow into the count badge by one column.
- **Paste to a closed pane no longer leaks per-pane state**, and the duplicate-name disambiguation search no longer reruns its O(projects²) scan on every paint tick — it's memoized on the actual project set.

### Added

- Unit tests locking the secondary-click modifier matching, the mouse-reporting guard, and the suffix-contained duplicate-name fallback.

## v0.57.0 — 2026-07-21

### Added — Fleet verification loop

- **Automatic verification per Fleet member.** Review can infer and run conventional validation commands for Cargo, Go, npm/pnpm/yarn, Python/uv, Maven, Gradle and .NET projects, or run an explicit command. Results persist locally with status, exit code, duration and a bounded log; timeouts terminate the full process tree.
- **Candidate scoring and ranking.** Review ranks Fleet members using verification status and change size, while showing changed files, line churn, untracked files, commits ahead, elapsed time and worktree health. Git sampling runs off-thread behind a short cache and never blocks the page.
- **Safe member retry.** Failed members can restart in their existing isolated worktree without losing committed, staged, unstaged or untracked changes. Attempts and launch errors are persisted with backward-compatible Fleet data.
- **Verification-gated merge.** A normal squash merge requires the member's latest verification to pass. MCP/CLI can explicitly use `force` to override the gate; the override and verification record are included in the audit response.
- **Web, MCP and CLI parity.** Added `review.verify`, `fleet.retry`, `unterm-cli review verify`, and `unterm-cli fleet retry`, plus Review controls for validation, logs, retry, rank and score.
- **Fast project and window navigation.** The left sidebar now groups tabs by a stable repository root, disambiguates equal project names with their parent directory, and exposes an always-visible fuzzy search. Results include tab number, Agent or foreground command, full project path and split count.

### Fixed

- **Windows checkpoint diffs include untracked files correctly.** The platform now passes exactly one null device when generating synthetic patches. Rollback verifies the restored Git tree, and merge responses include commit and staged-file audit data.
- **Right-click copy and paste target the clicked pane again.** Split-pane clicks no longer re-resolve to a stale active pane, and Windows clipboard reads once again start on the event-loop thread instead of silently failing on a worker.

### Changed

- **New command-loop brand mark.** The app icon and every logo surface (macOS .icns, Windows .ico, Linux hicolor ladder, sidebar / titlebar marks, website favicons and social card) move to the command-loop terminal mark — a metal U-loop with a jade prompt chevron on the dark tile.

## v0.56.0 — 2026-07-21

### Changed

- **The documentation caught up with the product.** README and unterm.app were rebuilt around Agent Cockpit as the headline: the MCP reference now documents the full surface — 97 methods across 21 namespaces (the `agent`, `cockpit`, `fleet`, `review`, `profile`, `meta`, `upload`, and `ghost` namespaces were previously undocumented) — and the CLI reference covers all 32 subcommand families, including the `fleet` and `review` families and the cockpit's `agent status / signal / inbox / enable-hooks`, each verified against the shipping binary. The site's capability map and per-namespace counts were corrected from the stale numbers (65 methods / 11 namespaces / 22 CLI commands).
- **No engine changes since v0.55.3.** This tag exists so the in-app version string, the update poller, and the website all point at the same milestone as the refreshed documentation.

## v0.55.3 — 2026-07-13

### Fixed

- **The "jump to bottom" button now actually scrolls to the bottom when clicked.** The button (added in v0.55) rendered correctly but its click handler was shadowed by an earlier catch-all match arm in the mouse dispatcher, so clicking it did nothing — only the keyboard `ScrollToBottom` worked. The dedicated handler is now reachable. (This also un-breaks the `-D warnings` CI check, which had been failing on the unreachable pattern since v0.55.1.)

## v0.55.2 — 2026-07-13

### Fixed

- **Windows: the minimize / maximize / close buttons no longer disappear when the window is narrowed.** In the top tab bar, the tab cluster carried a higher paint order (`zindex 1`) than the floated window-control cluster (`zindex 0`). A single tab is sized up to `window_width / tab_count`, so on a narrow window it extends across the bar and painted over the window controls, hiding them. The window-control cluster now paints on top (`zindex 2`), and click hit-testing already favors it, so the buttons stay both visible and clickable at any width.
- **Long login URLs (e.g. Claude Code's OAuth link) are now clickable and copyable in full.** Logical-line reconstruction capped a wrapped line at 1024 characters, which split URLs longer than that: clicking opened only the fragment under the cursor, and copying a selection that crossed the split inserted a newline mid-URL, breaking the paste. The cap is raised to 8192 — enough for any real login URL, still guarding the megabyte-JSON pathological case, and only computed on hover / selection.

## v0.55.1 — 2026-07-13

### Fixed

- **Windows: Claude Code no longer errors after every turn.** Claude Code executes its hooks through bash (Git Bash) even on Windows, and the cockpit hook command v0.55.0 wrote — `C:\Program Files\Unterm\unterm-cli.exe …` — was unquoted, so bash split it at the space and ate the backslashes, failing with `/usr/bin/bash: line 1: C:Program: command not found` at the end of every turn. The hook command is now quoted and uses forward slashes (valid in bash, PowerShell, and cmd alike). Re-running `setup-ai` (which the GUI does automatically on a version bump) or `unterm-cli agent enable-hooks` self-heals an existing broken entry in place rather than appending a duplicate. Only Claude Code and Aider were affected; Codex's `notify` is an argv array executed directly and was always fine.

### Added

- **"Jump to bottom" button on scrolled-away panes.** When a pane's viewport is scrolled off the live tail — reading history while an agent keeps streaming output — a small chevron button appears in the pane's bottom-right corner. Click it to return to the bottom and resume following; it disappears once the pane is pinned to the tail again. Keyboard `ScrollToBottom` is unchanged; this is the mouse-flow counterpart.

### Changed

- **The Web Settings page shows the release version, not just the build stamp.** It now reads `v0.55.1 (20260713-…)` — the semantic version you recognize first, the git build stamp (for bug reports) in parentheses — instead of only the raw stamp, which read as "not updated" right after installing a new release.

## v0.55.0 — 2026-07-11

### Added — Agent Cockpit

Unterm is the terminal AI agents can drive. v0.55 adds the other half: the terminal now sees, aggregates, and orchestrates the agents running inside it.

- **Agent state, live, everywhere.** Unterm watches every pane for AI agents (Claude Code / Codex / Gemini CLI / Aider / …) and folds four signal layers — official lifecycle hooks, OSC title/progress/notification parsing, foreground-process detection, and screen-text heuristics — into one state per pane: working, needs-you, done, idle. Zero configuration for Claude Code and Gemini; `unterm-cli agent enable-hooks` (run automatically by setup-ai) wires exact hook reporting for Claude Code, Codex, and Aider, merge-only with `.unterm-bak` backups.
- **You can see it without looking for it.** Sidebar tabs carry a state dot (breathing blue = working, amber = needs you, green = done); the top bar shows a cross-window tally chip that turns amber the moment any agent blocks on you. Click it — or press `Ctrl+Shift+A` — for the **Agent Inbox**: every agent sorted waiting-first, Enter jumps straight to its pane, wherever it lives.
- **Fleet: one task, N agents, N isolated worktrees.** Launch from the Inbox (or `unterm-cli fleet launch --agents claude,claude,codex -- <task>`): each member gets its own git worktree beside the repo (`../<repo>.fleet/…`), its own branch, and its own tab whose badge tracks that member's state. Crew presets are built from the agents actually installed; `claude+codex+gemini` runs the same task through three different models.
- **Review: nothing an agent does is untracked.** The moment an agent starts working in a repo, Unterm takes a non-invasive checkpoint (a dangling commit — HEAD, index, and files untouched). The new **Review** page in Web Settings shows every fleet member and checkpoint with line-level diffs (untracked files included), squash-merges a member's work into your repo as staged changes (the commit stays yours), discards what you don't want, rolls a repo back to any checkpoint, and compares two members' takes side by side.
- **Everything above is also MCP + CLI.** Twelve new methods (`agent.status/signal`, `cockpit.inbox`, `fleet.*`, `review.*`) and matching `unterm-cli agent status/signal/inbox`, `fleet`, and `review` subcommands — external agents can drive the cockpit itself.

### Fixed

- **CJK input methods no longer make palette inputs look dead.** With an IME active, keystrokes enter composition and never arrive as key events — palettes received nothing while the marked text painted at the pane cursor *behind* the card. Composition now previews inline in the palette's input line (tinted until committed) and the IME candidate window anchors to it. Applies to the fleet palette and the directory-jump palette.
- **OSC 9 / OSC 777 notifications now actually reach the window.** They were dropped by the mux-subscription pre-filter before any window-side consumer could see them.
- **MCP methods accept the documented `pane_id` parameter** (previously only `id` / `session_id` worked).
- **`unterm-cli agent signal` routes to the instance that owns the calling pane** (via the instance-unique `gui-sock-<pid>` in the pane's environment) instead of whichever instance registered last — with several windows open, hook events could tag the wrong pane.
- **Sidebar no longer rebuilds on every agent title-spinner frame.** Agents animate a braille spinner in their pane title several times a second, and the raw title sat in the sidebar cache key — every frame forced a full (~44ms) sidebar rebuild for as long as an agent worked. State glyphs are now stripped before the title enters the cache key.

## v0.54.5 — 2026-07-10

### Fixed

- **Ctrl+Shift+O now actually opens the directory-jump palette.** The shortcut was declared on the command definition but the action was never listed in the default-actions table that the key map is generated from, so the binding never existed and the keystroke fell through to the shell as `^O`. The palette was reachable only from the toolbar folder button and the context menu; the documented shortcut now works.
- **macOS/Linux: the bottom bar no longer mislabels the shell as `zsh (wsl)`.** The WSL heuristic was "the foreground process path starts with `/`" — which is true for every shell on unix platforms (`/bin/zsh`), so the `(wsl)` suffix appeared everywhere. The heuristic now applies only on Windows, where it correctly marks shells running inside WSL.

## v0.54.4 — 2026-07-10

### Fixed

- **The top-bar stats (cpu / mem / uptime / git) now update in real time.** They were frozen at whatever the first paint sampled, for two stacked reasons: the periodic status timer only re-arms inside the title-update path, which nothing reaches without a Lua status handler — so the timer died after one tick; and the stats text isn't part of `TabBarState`, so even a live timer never noticed it changing. The status tick now drives the title update directly, and the stats text participates in the chrome invalidation check. The bar repaints only when a value actually changes (~every 2s while values move, never when idle).
- **Windows: CPU% in the top bar shows a real value instead of a permanent 0.0.** The old sampler was a PowerShell `Get-Process` shim (a hidden ~100ms process spawn per refresh) that couldn't compute a percentage at all. It's now native Win32 (`GetProcessTimes` deltas between refreshes + `GetProcessMemoryInfo`), so the value is real and the refresh is effectively free.
- **Thin lines and small chrome text no longer smear or vanish.** Vertically centering an even-height child in an odd-height container produced a half-pixel offset, and the GPU's linear sampling spread every 1px stroke across two rows at ~50% alpha — the maximize button lost its top edge entirely and the stats text read as clipped. Middle-alignment now rounds to whole pixels.
- **Windows: the directory-jump palette's deep-scan rows indent correctly.** Relative paths were built with `\` separators on Windows while the depth indent counts `/`, so every nested row rendered at depth 0. Paths are now normalized to `/` everywhere.
- **The directory-jump palette no longer paints past its right edge on long paths.** Deep-scan labels and the dim path hints aren't clipped by the layout, so an over-long path pushed the text — and the selected-row highlight with it — outside the card. Labels and hints are now ellipsized from the left (`…window/render`) to the card's width.
- **The directory-jump palette shows one cursor, not two.** The mouse-hover highlight and the keyboard selection were independent, so hovering one row while arrowing to another showed both at once. Hover now moves the selection itself (launcher-style), so mouse and keyboard share a single cursor.

### Changed

- **The sidebar↔bottom-bar corner uses the same dark seam as the top chrome.** The bottom bar's top hairline was a light foreground-alpha line that disappeared against the sidebar's grey; it now uses the pane-background tone, matching how the top bar's seam is drawn, so the chrome frame reads consistently.

## v0.54.3 — 2026-07-08

### Fixed

- **No longer crashes on launch on GPU-less hosts (VM / RDP / cloud Windows / old GPUs).** When the OpenGL front end is rejected and Unterm falls back to WebGpu, the initial surface configuration used the window's dimensions verbatim — but configuring a wgpu surface with a 0 width or height panics ("Invalid surface"). On a GPU-less host the window can report 0 dimensions at that moment (the WARP software adapter, an unrealized window), so Unterm aborted before the window appeared. The initial surface size is now clamped to at least 1×1 (matching the existing guard on the resize path); the real size is applied as soon as the window is measured.

## v0.54.2 — 2026-07-06

### Fixed

- **Fullscreen no longer lets the macOS menu bar cover the terminal's top chrome.** In non-native (simple) fullscreen the window covers the whole screen, and the menu bar was set to auto-hide — so moving the mouse to the top revealed it as an overlay that covered Unterm's own top row (stats + toolbar buttons: settings, search, split, …), making them unreadable and unclickable. The menu bar and dock are now fully hidden in fullscreen for a true immersive mode, so they can't reveal on hover. macOS only.

## v0.54.1 — 2026-07-06

### Fixed

- **Pasting no longer freezes the window for up to half a second.** macOS pasteboards are lazy: reading the clipboard makes a synchronous cross-process request to whichever app owns it (a browser, editor, or IM), and that read ran on the GUI thread — so pasting content copied from a busy or slow source app stalled the whole window until that app answered. The clipboard is now read on a background thread. Text copied inside the terminal was already fast and stays fast.
- **Double-clicking a word no longer copies an extra character.** The mouse pixel-to-cell mapping rounded the click position to the nearest cell boundary — a forgiveness meant for drag-selection endpoints that also applied to a fresh click, so double-clicking the right half of a character rounded into the next cell and the word selection grabbed one extra letter (`hello` → `hellow` / `ohello`). A click now resolves to the cell the pointer is actually inside; dragging still rounds so the selection endpoint stays forgiving.

## v0.54.0 — 2026-07-05

### Performance

- **Cold start is 2.8× faster: ~780ms → ~280ms to first frame.** Five startup-path optimizations: config keys are validated against a precomputed key set instead of materializing the whole config per key (Lua eval 297ms → 8ms); the second config load is skipped unless the GUI actually queried screens/appearance before startup (128ms → 0); WSL distros are enumerated from the registry instead of spawning `wsl.exe` (43ms → 4ms); the 1119 built-in color schemes parse in parallel and warm in the background (195ms → 43ms); and the OpenGL context is prewarmed on a helper thread right after argument parsing, then adopted by the real window (GPU setup 222ms → ~110ms, fully overlapped).
- **Sustained output no longer burns a CPU core on the UI thread.** During output floods (hundreds of repaint requests per second), the throttled `WM_PAINT` handler returned without validating the update region, so Windows re-queued `WM_PAINT` continuously and the message loop spun at ~91% of a core. It now validates the region and lets the frame-rate timer re-invalidate; the UI thread drops to ~4% during the same flood.
- **The MCP server stays responsive during output floods.** Same root cause as above: MCP operations that need the main thread were starved by the paint storm, so `unterm-cli` calls timed out after 30s mid-flood. They now answer in tens of milliseconds.

### Fixed

- **The first terminal row is no longer clipped under the top bar.** The reserved chrome height was still the old 1.6× title-cell formula while the quick-action buttons had grown the painted chrome to ~2× + divider, hiding the top ~16px of row one. The reserved height is now derived from the same button-geometry constants the layout uses, so they can't drift apart again.
- **The settings-menu prewarm no longer fails on small glyph atlases.** The menu's codicons/CJK labels could outgrow the startup atlas; the prewarm now grows the atlas in place (like the paint pass does) instead of giving up, which also spares the first real menu open the atlas-growth hitch.
- **CLI commands without `--id` now target the pane you're looking at.** `session.list` exposed an unordered HashMap walk and the CLI picked its first entry, so with several panes open a bare `unterm-cli exec run` could type into a random pane. The list is now sorted with an `is_active` flag and the CLI prefers the active pane.
- **`session.resize` refuses panes that are laid out by the GUI window** instead of silently desyncing the PTY grid from the visible grid (content clipped / wrapped wrong until the next window resize).
- **A mistyped CLI flag can no longer end up typed into your terminal.** Trailing command/text arguments no longer accept unknown flag-like tokens (`--sesion 0 ...` previously flowed into the command string and was sent to the live pane); clap now rejects them with a hint to use `--`. Flags after the first command token (`ping -n 1 ...`) and the documented `-- --flag` escape are unaffected.

## v0.53.2 — 2026-07-04

### Fixed

- **The left sidebar no longer covers the bottom status bar.** The sidebar's trailing `+` / shell-picker row overflowed its surface and dragged the whole panel down over the bottom info bar, hiding the shell name and the start of the working directory. The footer is now reserved inside the sidebar's height so it meets the status bar flush.
- **Double-clicking a directory in the tree opens the right one.** The first click of a double-click expands the directory and repaints, shifting every row below; the second click then hit-tested a shifted row and `cd`'d to the wrong directory (intermittently, as a repaint race). The double-click now acts on the path captured by the first click and undoes that click's expand.
- **The hover/selection highlight no longer lands on two rows at once.** The hover hit test used an inclusive interval, so two vertically-adjacent rows both matched on the pixel they share. It now uses a half-open interval, so each row owns its top edge but not its bottom edge.
- **macOS traffic-light dots are the right size on Retina.** The custom-drawn window buttons were sized in raw device pixels, rendering at half the native cap diameter on 2× displays. They now scale with DPI.
- **The ↔ resize cursor no longer appears over the tab rows on HiDPI.** The sidebar resize hit test compared the physical mouse position against a logical edge coordinate as a fallback, which matched in the middle of a tab row on 2× displays. Only the (correct) physical test remains.

### Changed

- **Clearer chrome across all themes.** The sidebar↔content and bottom-bar↔content edges use a clearly-visible hairline instead of tone-only contrast, and the top-bar action icons are brighter, so divisions and buttons read cleanly on the dark themes.

## v0.53.1 — 2026-07-03

### Fixed

- **Windows no longer crashes on startup on GPU-less / old-GPU machines.** When the initial software OpenGL front end is rejected and Unterm falls back to WebGpu, the render backend is briefly torn down and rebuilt. The background settings-menu prewarm could fire during that window and dereference the absent render state, aborting the process before the window ever appeared (seen on Windows-on-ARM and in VM / RDP sessions). The prewarm now backs off cleanly and the menu builds once the WebGpu backend is live. macOS and Linux were unaffected.

## v0.53.0 — 2026-07-03

### Added

- **Composer — a prompt queue for driving agents (`Ctrl+Shift+J`).** Queue up several prompts and run them into the active pane one after another. In auto-approve mode the Composer smart-advances through an agent's confirmation prompts, so a batch of instructions runs to completion without babysitting each step.
- **Read-only Git panel (`Ctrl+Shift+G`).** A right-docked panel shows the active pane's repository at a glance — branch, upstream tracking, and working-tree status — without shelling out. The terminal reflows around it, and it toggles away just as fast.
- **Distinct left-sidebar tab titles.** Each sidebar row now derives its label from the pane's foreground command, so a window full of shells reads as its actual work (`zhitong@host`, an editor, a build) instead of a column of identical `zsh` entries.
- **`USAGE.md` agent-terminal guide** documenting the Composer / prompt queue, the Git panel, and the tab-title behavior.

### Changed

- **Unified chrome.** The top bar, left sidebar, and bottom status bar now share one continuous frame and tone (top-left Unterm wordmark, frameless top-bar buttons, no redundant dividers), so the window reads as a single surface rather than three stacked strips.
- **Otty-style left tab bar.** The active row gets a rounded accent-colored selection, rows carry quiet activity indicators, and group headers are cleaner.

### Fixed

- **The left sidebar no longer covers the bottom status bar.** The sidebar's trailing `+` / shell-picker row was overflowing its surface and dragging the whole panel down over the bottom info bar, hiding the shell name and the start of the working directory. The footer row is now reserved inside the sidebar's height, so it meets the status bar flush and the info bar shows in full from the left edge.
- **CJK locales no longer garble menu, overlay, and shell-selector cards.** Wide-character width is measured in cells, so cards laid out under a Chinese locale stop showing mojibake.
- **Windows: no console-window flashes at startup**, a **process-lock self-deadlock** that could hang the window from ever showing is resolved, and the status-bar working directory no longer carries a stray remote-host prefix.
- **`Ctrl+Shift+J` reliably opens the Composer** — the toggle is now registered in the command list.
- **Less tofu flash on first paint.** The top-bar wordmark, tree-sidebar glyphs, title font, and directory-jump glyphs are prewarmed so they render immediately instead of flashing missing-glyph boxes.

### Performance

- **Smoother sidebar resize.** Dragging the sidebar edge now throttles the PTY reflow to ~25 fps instead of reflowing on every pixel of the drag.

## v0.52.0 — 2026-06-29

### Added

- **More AI agents recognized out of the box.** The baked agent manifest now ships definitions for **Kimi Code CLI** (Moonshot login or BYO key) and **Trae Agent** (provider / model / config-driven), so they are auto-discovered and registerable like the existing agents without a manifest refresh.

### Fixed

- **Windows windows can always be grown again.** The window minimum size is now clamped so a window can no longer be shrunk to a tiny size that left it impossible to drag back larger.
- **Windows clipboard survives transient contention.** Copy/paste now retries when another process briefly holds the Windows clipboard open, instead of failing the operation outright.

### Performance

- **Fewer GUI stalls.** The left tab bar, status bar, top stats bar, ghost-text, and pane paint paths were reworked to cut per-frame work, keeping the UI responsive under load.

## v0.51.1 — 2026-06-20

### Fixed

- **Directory-jump finder no longer leaves a big empty gap above the results.** When the finder had more than one page of matches, the scrollbar — a tall floated block placed before the rows — pushed every result and the footer down by its full height, so the matches sat at the bottom under a large blank band. The rows now sit directly under the search field with the scrollbar floated alongside them.

## v0.51.0 — 2026-06-20

### Added

- **Native macOS glyph weight (`font_smoothing`).** The CoreText rasterizer now applies the system's grayscale font smoothing by default, so terminal and chrome text pick up the same stem-darkening as Terminal.app instead of rendering thin. New `font_smoothing` option (default on); set it to `false` for the lighter Warp-style HiDPI look. macOS only.

### Changed

- **Sidebar typography breathes.** The chrome UI font is now 13.5pt with looser row leading, so left-tab labels read larger and less cramped. The bottom footer is a single quiet "DO AI PM" caption, with a generous gap above the add-tab `+` row so it is no longer mis-clicked.

### Fixed

- **Split divider draws all the way to the edge; pane close button sits in the corner.** The vertical-split divider now spans the full pane width to the window border instead of stopping a few pixels short, and the per-pane close `×` moved out of the chrome gutter onto the pane's first content row, tucked against the top edge and the divider.

### Performance

- **Left sidebar scales to many tabs.** Per-tab metadata (title / agent / working directory) is gathered only for the visible rows instead of for every tab on every repaint, keeping windows with dozens of sessions responsive.

## v0.50.2 — 2026-06-18

### Fixed

- **Theme switching now repaints the active input row immediately.** Runtime theme changes update the window config, terminal palette, and line render caches together, so the prompt no longer stays in the previous theme until Enter is pressed.
- **Bottom status text is vertically centered.** The status bar now accounts for its own vertical padding in layout and paint, avoiding the old low-sitting text.
- **Terminal appearance polish.** Refined built-in theme palettes, sidebar spacing, active-tab contrast, and the live settings theme switch path so the UI reads more consistently across light and dark skins.

## v0.50.1 — 2026-06-17

### Fixed

- **macOS Chinese / IME input no longer flashes garbled text before settling.** The Cocoa string bridge now reads the actual UTF-8 string instead of truncating by UTF-16 code-unit length, so composed CJK text is rendered with complete bytes from the first frame.

## v0.50.0 — 2026-06-17

### Changed

- **Official site now tracks the current release.** The homepage hero badge, footer version chip, latest download URLs, and release fallback all point at `v0.50.0`, so visitors and build-time fallbacks stay aligned with the published tag.

## v0.44.4 — 2026-06-16

### Fixed

- **Left sidebar selection now reads as one surface instead of stacked overlays.** The active row uses a deeper neutral fill, the cyan marker is inline with the row content, and the outline was removed so the selection no longer looks like layered gray boxes.
- **Scrollbar interaction is reliable again.** The left sidebar scrollbar now sits flush against the edge, the thumb track matches the visible rows, and wheel / drag input stays responsive without fighting the row layout.
- **New tabs activate cleanly.** Spawning a tab no longer does a redundant second activation pass, which keeps focus and scroll position stable.

## v0.44.3 — 2026-06-15

### Fixed

- **Visible seam between the top bar and the left sidebar.** The
  v0.44.1 / v0.44.2 attempts (1.3× / 1.0× chrome height) both broke
  worse than they fixed: shortening the chrome clipped the macOS
  traffic lights and the terminal's first row, and the user's
  recurring complaint ("边栏压住顶栏") wasn't a geometry overflow at
  all — the chrome's background and the sidebar's background both
  resolved to near-identical greys (sidebar bg lifted only 5 % over
  chrome bg), so the boundary between them was invisible and the
  chrome's empty lower region read as a continuation of the sidebar.
  Restored chrome to 1.6× cell_height (the size everything was
  designed around) and added a 1 px divider line at the chrome's
  bottom edge in `bar_fg * 0.12` alpha — matches the divider already
  drawn at the sidebar's right edge, so the chrome / sidebar /
  terminal panels each read as their own surface.

## v0.44.2 — 2026-06-15

### Fixed

- **macOS chrome dead band below the traffic lights.** v0.44.1's chrome
  was still 1.6× cell height (~50 px) — the AppKit native traffic
  lights anchor to a fixed ~14 px y-offset from the window's top edge
  regardless of chrome height, so the lights sat at the top with ~22 px
  of empty bar below that nothing filled. Dropped the multiplier to
  1.0× cell height (~31 px) so the chrome matches the lights' natural
  row; codicons + stats text now share the same row as the lights and
  the sidebar's first tab sits flush against the chrome's bottom edge
  with no visible gap.

### Changed

- **Default `integrated_title_button_style` back to `MacOsNative` on
  macOS.** v0.44.1 had flipped the default to `MacOsCustom` (the new
  custom-drawn dots) to chase pixel-exact centering, but that lost the
  AppKit hover glyphs (X / − / +) and the rendered dots looked flatter
  than the OS lights. Reverted to `MacOsNative`; `MacOsCustom` stays
  available for users who explicitly want it via `unterm.lua`.

## v0.44.1 — 2026-06-15

### Added

- **`IntegratedTitleButtonStyle::MacOsCustom`** — three filled circles
  (Apple-palette red / yellow / green) drawn through our own box-model,
  so `VerticalAlign::Middle` lands them at pixel-exact chrome center.
  AppKit's native NSWindowButton widgets are anchored to a fixed
  y-offset from the window's top edge, not the chrome center, so the
  unified Warp-style top bar always left the lights off-axis (with a
  dead band either above or below depending on chrome height). The
  custom dots route clicks through the existing `TabBarItem::WindowButton`
  handler — close / minimize / zoom work identically. Default on macOS
  flipped from `MacOsNative` → `MacOsCustom`; set
  `integrated_title_button_style = "MacOsNative"` in `unterm.lua` to
  go back to OS-drawn lights (which include the X / − / + hover
  glyphs that the custom dots don't reimplement).

### Fixed

- **Stats segment overlapped the traffic lights on narrow windows.**
  The right-aligned stats text + codicon cluster floats from the right
  edge; as the window shrank past ~900 px the codicons reached the
  traffic-light reservation and the stats text started drawing under
  the lights. The stats composer now early-returns empty when
  `pixel_width < 900` — codicons stay (they're controls, not status)
  and the stats segment re-appears as soon as the window widens past
  the threshold.

## v0.44.0 — 2026-06-15

### Added

- **Per-pane top stats bar.** The tab row now carries an inline status
  strip for the active pane: branch + ahead/behind from `git status`,
  the foreground process name, its CPU / RSS / uptime, and the active
  MCP agent if one is driving the pane. Git and process info refresh
  off the render thread on a short cache so typing isn't slowed; both
  `ps` parsing (Unix) and a PowerShell `Get-Process` shim (Windows)
  feed the same pipeline. Agent detection now walks the pane's full
  process tree, so `claude` / `codex` / `gemini` get picked up even
  when they're spawned underneath a tmux or shell wrapper.
- **Sidebar footer linking to doaipm.com.** A pinned link row at the
  absolute bottom of the left tab bar — `DO AI PM · BY ZHITONG ↗` in
  the platform UI font (SF Pro on macOS) — opens the project's
  marketing site via the system browser. Caption is localized across
  all 9 shipped locales (`sidebar.author_caption`) and rendered as a
  separate, z-pinned element so it doesn't follow the tab scroll.
- **`▾` shell selector in the left tab bar.** Next to the trailing
  `+` row, a discoverable arrow opens the shell picker (login shells,
  registered profiles, AI agents) so new tabs no longer require the
  context menu. Both hit targets sized for comfortable click.
- **`tree '↑ ..' row` for parent navigation.** In tree-sidebar mode
  the first row is now a real "up one directory" entry — click to
  re-anchor the tree at the parent. Previously the only way out was
  to type the parent path manually.

### Changed

- **Chrome height + cluster geometry tightened.** Multiple passes over
  the unified top bar: traffic-light placement reuses `ui_tokens`
  instead of guessing macOS widths, the bar is centered vertically,
  Codicon spacing is even, the duplicate tab-title in sidebar mode is
  dropped, and the overall chrome height comes down ~15% without
  losing finger-target area.
- **Stats / status / tree-sidebar accents derive from the theme's
  palette.** The chrome's teal accent (clickable chips, git ahead/
  behind, process CPU/MEM) now reads from `palette.brights[6]` /
  `palette[14]` so it tracks the active scheme instead of being a
  hard-coded `#6fccb8`. Top-bar background lightens 5% over the
  scheme's bg so the chrome lifts off the pane.
- **`session.create` saved state is per-binary.** Workspace restore
  now keys session JSON on a sha1 of the running binary path, so
  the installed `Unterm.app` and the dev binary at `/tmp/Unterm-dev.app`
  don't trample each other's tabs on simultaneous use.
- **Left vertical tab bar** (`tab_bar_position = "Left"`). Tabs become
  rows in a resizable sidebar (drag the right edge, 200pt–50% window):
  each row shows the tab title over an `agent · directory` subtitle —
  which MCP agent is currently driving that pane (15-minute freshness)
  and the active pane's directory. Click activates, dragging reorders
  live, double-click renames inline (empty resets to auto-title),
  right-click opens the tab context menu, ✕ closes, a trailing `+` row
  spawns, and the wheel scrolls. The top bar stays for window buttons,
  the active title, the menu and quick actions. Toggleable from the
  View / window menus; defaults to the classic top strip.

- **Search button in the tab bar.** A magnifying-glass quick-action button
  next to tree / split / dir-jump / settings opens the scrollback search
  for the active pane — same as `Ctrl+Shift+F` / `Cmd+F`, but
  discoverable. Search was previously reachable only via keybinding or
  the Edit menu.
- **`screen.search` can now jump to its results.** New `goto: true` /
  `goto_match: N` params scroll the user-visible viewport so the match is
  on screen (with a quarter-viewport of context above it) — search-and-
  locate instead of search-and-report. Matches now carry the stable row
  index (same coordinate space as `screen.scrollback_text`) plus the
  match column, so agents can address results even as new output streams
  in. Previously the returned `row` was a bare enumeration index that
  drifted once the scrollback was trimmed, and there was no way to bring
  a match into view. The in-terminal search UI (`Ctrl+Shift+F` /
  `Cmd+F`) already jumped to matches; this brings the MCP surface to
  parity.
- **`unterm-cli screenshot --self`.** Capture only Unterm's own window
  via its CGWindowID — independent of foreground state, never depends
  on what's behind the window. Goes through the existing `capture.window`
  MCP method (defaults pid to the running server) so it works without
  any osascript / activate dance, which lets visual self-tests run
  headlessly. Pairs with the agent-side self-test workflow the global
  CLAUDE.md asks every client project to ship.
- **Workspace restore reopens your tabs, not just the window.**
  Closing Unterm with three tabs no longer comes up with a single
  fresh shell at the default cwd. Tabs beyond the first are spawned
  back into the restored window, each pointing at the cwd they were
  at on shutdown, and any user-set tab titles ride along. Falls back
  to the default cwd if the saved directory has since been deleted
  (no "no such file" bounce). Surfaces per-tab spawn failures via
  log only — one bad tab doesn't abort the rest.

### Changed

- **Unified Warp-style top bar by default.** The split chrome (gray
  native title strip above + dark tab row below) collapses into a
  single panel that adopts the active color scheme — traffic lights
  drop into the tab row, the bar tracks `palette.background` so the
  chrome no longer fights the theme, and height bumps to 2.4× the
  title cell (~60 px @ 144 dpi) to match Warp's chrome rhythm. On the
  right of the bar a six-icon action cluster + `▾` menu lands: command
  palette, tree sidebar, split, dir-jump, search, settings. All icons
  are Codicons rendered through the bundled SymbolsNerdFontMono so
  CoreText/FreeType handles the anti-aliasing — vector-grade crispness
  instead of the previous geometric polylines. Tabs now read
  `[icon] {agent or shell} · {cwd-basename}` (claude / shell / ssh
  glyph, AI agent name preferred over raw process title), and the left
  vertical tab bar's status dot becomes a 16 px circular chip with the
  agent's initial in its accent color. Users who had explicitly
  overridden `window_decorations` or `window_frame.titlebar_*` colors
  keep their overrides — the new defaults only kick in when the field
  is at its hard-coded value.
- **Chrome typography uses the platform system font.** Tab bar, sidebar
  and menu text now renders in SF Pro on macOS (Segoe UI Variable on
  Windows, Noto Sans on Linux) instead of the bundled Roboto, with the
  scale collected into `config::ui_tokens`. macOS text rendering also
  switches from subpixel-LCD to grayscale anti-aliasing — macOS removed
  subpixel AA system-wide in Mojave, so the old default produced visible
  red/blue fringing on Retina panels next to natively rendered text.
- **Search bar UX revamp.** The in-pane search bar (toolbar 🔍 /
  `Ctrl+Shift+F`) is no longer a cryptic English-only one-liner:
  - Localized in all 9 languages, with a dim "type to search…"
    placeholder and the match count + match mode right-aligned so they
    don't jitter while you type.
  - Key hints are shown in the bar itself (`Enter/↑↓ jump · Ctrl+R
    mode · Esc close`) — previously none of the search keys were
    discoverable. Hints degrade gracefully on narrow panes.
  - `Shift+Enter` now jumps in the opposite direction of `Enter`.
  - Interactive search now defaults to **ignore-case** (Ctrl+R still
    cycles exact / regex). Agent-driven `screen.search` is unchanged.
  - Typing latency: the per-keystroke search debounce dropped from
    350 ms to 100 ms, so first highlights appear ~3× sooner.
  - The pattern restored from your last search (or seeded from the
    selection) is replaced wholesale by the first character you type,
    instead of being appended to — backspace/arrows still edit it.
  - The bar now opens instantly: the seeded-pattern re-search is
    deferred off the open path, live pane output re-triggers the scan
    at most twice a second instead of every frame, and the chunked
    scan no longer queues a full window repaint per 1000 rows.
  - Switching matches centers the hit in the viewport (it used to hug
    the screen edge), and the active match is bold + underlined on top
    of its highlight color so it stands apart from the other matches.
  - The search-box cursor blinks, so the typing position is findable
    on the reverse-video bar.
  - **Search opens in one frame, every time.** Two more delays found by
    instrumenting the open path: (1) the copy/search overlay never
    reported its own dirty rows through `get_changed_since`, so the
    renderer's line cache treated the bar, the highlights, and the
    active-match change as "nothing changed" — the search UI literally
    waited for an unrelated repaint; keystrokes now also dirty the bar
    row directly, so typing echoes instantly instead of after the
    search debounce. (2) The localized bar's CJK + ↑ ↓ · glyphs paid
    a few hundred ms of first-time font-fallback resolution exactly on
    first open; those glyphs are now pre-shaped at idle right after the
    window opens. Measured open-to-painted: 33 ms cold (was 230 ms+,
    occasionally seconds).
  - **Overlays paint immediately.** Opening search (or quick select /
    any pane overlay) never requested a repaint, so the bar waited for
    the next incidental one — a cursor blink or pane output — which is
    why it felt slow no matter how fast the search itself got.
  - **Click a match to jump to it.** Left-clicking any highlighted
    match makes it the active one and centers it; previously a click
    silently wiped the match selection. Clicks elsewhere still do
    normal text selection. Inactive matches render dimmed so the
    active one stands apart.

### Fixed

- **`unterm-cli screenshot --self` no longer falls back to a
  full-screen capture right after launch.** Immediately after
  `unterm start`, the NSWindow exists but CGWindowList hasn't yet
  flagged it onScreen, so the lookup returned None and the MCP
  handler silently captured the screen instead — agents doing visual
  self-tests would diff against whatever was behind Unterm. For
  self-targets the lookup now retries 5× with 120 ms gaps (~600 ms
  ceiling). External-pid captures still single-shot so a typo'd pid
  doesn't block.
- **Windows: multi-second freeze at launch (and on every new tab) when the
  proxy toggle is on but the proxy app isn't running.** System-proxy
  auto-detection ran twice on the GUI thread before the first prompt, and
  each pass swept 8 well-known local proxy ports serially — on Windows a
  TCP connect to a closed loopback port only fails after winsock's internal
  retry, so every closed port ate its full 120 ms timeout (~1 s per sweep,
  ~2.5 s total). Detection stages now run concurrently, the port sweep is
  parallel, and results are cached for 5 s and shared by the startup,
  spawn, ▼ menu, and Web Settings paths. Worst-case cold detection is now
  ~150 ms; spawns within the cache window are free.
- **Proxy auto-detection no longer scans ports 8080 / 8888.** These are
  far more often dev servers than proxies, and the scan's only signal is
  a successful TCP connect — a dev server on 8080 would be "detected" as
  a proxy and injected as `HTTP_PROXY` into every spawned shell, cutting
  that shell off from the network. Proxies genuinely listening there can
  still be configured explicitly in `proxy.json` (manual mode).
- **Top stats bar: invisible-render bugs.** Two issues at once kept the
  inline strip blank on first paint: the cell width was computed as
  `pixel_cell = 0` (the strip collapsed to zero-width before text was
  measured), and the strip's z-index sat under the pane so even a sized
  strip was painted over by terminal content. Width now uses the real
  cell metrics; the strip is composed into the chrome row itself
  (zindex 20) so it draws above the pane and behind the click targets.
- **`ps` output parsing in the stats bar handled runs of whitespace
  incorrectly.** `splitn(4, char::is_whitespace)` emits empty fragments
  for runs of spaces, so the RSS column landed in the comm slot and
  the process name showed up as a number. Switched to
  `split_whitespace()`, which collapses runs the way macOS / Linux
  `ps` formats actually want.

## v0.16.0 — 2026-05-18

Two-stage AI-friendly terminal release: locks down the MCP write
surface, adds proposal-style AI integration, and ships fish-style
inline ghost text plus a read-only insights panel.

### MCP security & audit (P0)

- **Every `session.input` / `exec.send` is audited**. The audit log
  records timestamp, method, pane id, content preview (truncated +
  escape-aware), and the calling agent's identity. Cap 1 000 entries
  (configurable via `mcp_audit_log_capacity`).
- **`agent.identify` / `agent.whoami`** — agents self-tag a
  connection so audit entries group by name instead of by socket.
- **First-time-per-agent confirmation banner**. When a new agent
  first writes to the PTY a blocking banner appears above the
  status bar:
  `⚠ <agent> wants to write to pane #N: <preview>` ·
  `[Enter] allow [Esc] block [Alt+A] always allow`.
  Worker thread parks until the user decides or the policy timeout
  (default 30 s) expires (auto-block). Policy configurable via
  `mcp_input_confirmation = Always | FirstTimePerAgent (default) | Never`.
- **Status bar chip `mcp:N`** shows cumulative MCP PTY writes. Click
  copies the recent audit log as JSON to the clipboard.
- New config: `mcp_input_confirmation`, `mcp_confirmation_timeout_ms`,
  `mcp_audit_log_capacity`, `mcp_suggest_queue_capacity`,
  `mcp_suggest_default_ttl_ms`, `mcp_trusted_agents`.

### MCP suggest API + UI (P1)

- **`session.suggest{,_status,_cancel,_list}`** — agents propose
  text that **never reaches the PTY directly**. Lifecycle:
  Pending → Accepted / Dismissed / Expired / Cancelled. Default
  TTL 60 s. Queue capacity 256.
- **Suggest bar UI** — single half-transparent row above the
  status bar showing `✨ <agent>: <text>` plus the keybind hints.
- **Default keybindings**:
  - `Tab` — accept suggestion (passes through to shell completion
    when no suggestion is pending).
  - `Esc` — dismiss suggestion / block pending confirmation
    (passes through to vim, less, etc. when neither is pending).
  - `Alt+Enter` — accept suggestion and append `\n`.
  - `Enter` — allow pending confirmation (passes through when no
    banner is up).
  - `Alt+A` — always-allow the calling agent for this session.
- Acceptance / dismissal is audited under
  `session.suggest.accept` / `session.suggest.dismiss`
  with `agent="user"`.

### Ghost Text (fish-style inline completion)

- Per-pane keystroke observer tracks the current input buffer.
  Inline grey italic continuation rendered to the right of the
  cursor when the buffer prefix-matches a previously-committed
  command.
- **Right Arrow / End** — accept the prediction (writes the
  continuation to the PTY). Returns `Unhandled` when no prediction
  is showing so the keys still move the cursor.
- **Cross-pane history pool** (512 commits) so freshly opened
  panes have something to predict from day one.
- **↑/↓ history navigation, ←/→, Tab, Home** all clear the
  ghost buffer to keep it in sync with what the shell rewrites.
- **OSC 133 prompt detection** — when shell integration is on,
  the overlay only renders inside `Input` semantic zones; TUI
  applications and command output are left alone.
- **`ghost.debug` MCP method** for inspecting per-pane buffer,
  prediction, and cross-pane commit pool from outside the GUI.

### Insights panel

- Read-only dashboard bound to **`Ctrl+Shift+I`** (or
  `KeyAssignment::ShowInsights`). Shows:
  - Shell, cwd, terminal size for the active pane.
  - Most recent 10 commands across all panes.
  - Top-5 most-typed commands (cross-pane frequency).
  - MCP write counters, time since last write, agents seen,
    pending suggestions / confirmations.
  - Last 8 audit log entries.
- Pure local data, no AI, no network round-trip.
- `q` / `Esc` / `Ctrl+C` to dismiss.

### Misc

- Status bar `mcp:N` chip click now copies up to 200 audit entries
  as JSON to the clipboard. The full overlay (Ctrl+Shift+A) is a
  follow-up.
- `agent_first_input` flag added to audit log entries when a new
  agent first hits a PTY-writing method, even when the policy
  doesn't block.
- Lock order documented: `registry()` (per-pane ghost state)
  before `global_commits()` (cross-pane pool). Future contributors
  please honor this — concurrent observe calls would deadlock if
  the order flips.

### Known limitations

- Ghost Text only learns from Enter-committed commands; commands
  recalled via shell history (↑/↓) don't enter the pool, so users
  who lean heavily on history navigation will see fewer
  predictions until they type a command at least once.
- The `Ctrl+Shift+A` audit log overlay is still placeholder — the
  status bar chip copies JSON to clipboard as a stand-in.
- AI Ghost Text (Claude API-driven completion) is intentionally
  not shipped — slated for v0.17.
