# Unterm — Agent-Terminal Features

Unterm is a Windows-first terminal built for driving AI coding agents. Below are
the features that set it apart, with how to use each. (Keys are the defaults.)

---

## 1. Composer + Prompt Queue — `Ctrl+Shift+J`

Draft several instructions up front and let Unterm feed them to an AI agent (or a
shell) **one at a time**, waiting for each to finish before sending the next — so
you can line up a multi-step job and walk away.

**How to use**
1. In a pane running an agent/shell, press **`Ctrl+Shift+J`** to open the Composer.
2. Type a prompt, press **`Enter`** to add it to the queue (repeat to stack several).
3. Press **`Ctrl+Enter`** to run — prompts are sent in order.
4. **`Esc`** / `Ctrl+S` stops; `Shift+Enter` newline; `↑/↓` select; `Del` remove; `Ctrl+K` clear all.

### Smart auto-advance — press `Tab` to switch mode
When a step finishes, how should the next one be sent? Cycle the mode with **`Tab`**
(shown in the footer):

- **Auto-approve (default)** — when the agent pauses, Unterm reads the screen:
  - a **Yes/No confirmation** → it auto-approves and keeps going (unattended);
  - a **real multi-choice** (3+ options) → it **pauses** so you pick, then resumes on its own;
  - **task done** → sends the next queued prompt.
  - When it isn't sure, it **pauses rather than guess**.
  - ⚠️ Auto-approve means the agent acts without you reviewing each confirmation — use it when you trust the queued run.
- **Auto-next** — always send the next prompt on ~600ms of silence (no inspection).
- **Manual** — pause after each; you trigger the next yourself.

> Example: queue "refactor this function", "now add tests", "now run the tests" —
> hit Run, and it walks the whole chain, clicking through Yes/No prompts for you.

---

## 2. Git Panel — `Ctrl+Shift+G`

See source-control status without typing `git status`.

Press **`Ctrl+Shift+G`** in a repo pane → a right-docked panel shows the current
**branch**, **ahead/behind** vs upstream, and **changed files** (staged /
unstaged / untracked). Press again to hide. (Read-only for now.)

---

## 3. Smart tab titles (automatic)

The left sidebar names each tab by **what it's doing**, not a generic shell name:
- running a command (`npm run dev`, `claude`, `git log`) → the tab shows that command;
- idle at a prompt → the shell name;
- an agent pane → the agent's name.

So a stack of shells stays distinguishable instead of ten identical `powershell · ~`.

---

## Handy extras
- **`Ctrl+Shift+O`** — fuzzy directory jump (go-to-directory from the pane's cwd).
- Left sidebar shows **activity dots** (unread output / running / error) per tab.

---

*Living doc — updated as features land. Source of truth for the website's
feature highlights.*
