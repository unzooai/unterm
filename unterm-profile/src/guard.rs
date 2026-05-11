//! Destructive-command guard — shell-side prompt before dangerous ops.
//!
//! The design doc (§11) originally called for shell preexec hooks
//! reporting commands over a local socket so Unterm's GUI could pop
//! a confirmation overlay. That's the right approach for v0.14 — it
//! gives the guard the same visual identity as the chip and works
//! regardless of which shell flavor the user runs. But it's also a
//! three-piece architecture (shell hook + Unterm socket listener +
//! GUI overlay), and any one piece breaking makes the guard silently
//! stop guarding.
//!
//! For v0.13 we ship a simpler approach: a single shell script the
//! user sources from their `.bashrc` / `.zshrc`. The script wraps a
//! curated list of destructive binaries (`gh`, `aws`, `npm`, `git`,
//! `vercel`) with shell functions that check `UNTERM_PROFILE` and
//! prompt before passing the call through. Pros:
//!
//! - **Self-contained**: works in any pane, no Unterm-side wiring.
//! - **Transparent**: vibe coder can `cat` the script and audit it.
//! - **Conditional**: guard only triggers when `UNTERM_PROFILE` is
//!   set, so non-Unterm shells (cron, ssh-into-server, …) are
//!   unaffected.
//! - **Bypass-friendly**: `command gh repo delete ...` short-circuits
//!   the function and runs the binary directly when the user wants
//!   to skip the prompt (e.g. in a known-safe script).
//!
//! The GUI-overlay path lands in v0.14 alongside the picker overlay
//! and Settings panel — at which point both surfaces coexist and the
//! user picks whichever fits their habits.

use anyhow::{anyhow, Result};

/// Script content for bash and zsh. Both shells use the same syntax
/// here — POSIX-ish function definitions, no array slicing, no
/// arrays, only stuff that works in both. We branch on `$ZSH_VERSION`
/// inside the script for the one place it matters (`read` flag).
pub const BASH_ZSH_SCRIPT: &str = r##"# Unterm destructive-command guard (v0.13)
#
# Prompts before destructive operations when this shell is bound to an
# Unterm identity profile. Source from your rc file:
#
#   [[ -n "$UNTERM_PROFILE" ]] && eval "$(unterm-cli profile shell-integration zsh)"
#
# (Or use `bash` in place of `zsh` — the script is the same for both.)
#
# Bypass for known-safe scripts: invoke the binary with `command <bin>`
# (e.g. `command gh repo delete unzooai/disposable`) — that skips the
# wrapper function and runs the real binary directly.
#
# The guard only triggers when $UNTERM_PROFILE is set. Outside Unterm
# (or in an un-bound window) all wrappers pass through transparently.

_unterm_guard_confirm() {
    # $1: short label for the operation
    # $2: full command line for context
    local label="$1"
    local full="$2"
    printf '\n\033[1;33m[Unterm guard]\033[0m profile=%s\n' "${UNTERM_PROFILE:-?}"
    printf '\033[1;33m[Unterm guard]\033[0m About to run: %s\n' "$full"
    printf '\033[1;33m[Unterm guard]\033[0m %s — proceed? [y/N] ' "$label"
    local reply=""
    if [ -n "${ZSH_VERSION:-}" ]; then
        # zsh's `read` doesn't take -r unless we tell it to
        IFS= read -r reply
    else
        IFS= read -r reply
    fi
    case "$reply" in
        y|Y|yes|YES) return 0 ;;
        *)
            printf '\033[1;33m[Unterm guard]\033[0m Aborted.\n'
            return 1
            ;;
    esac
}

# --- gh: repo delete + repo archive ---
gh() {
    if [ -z "${UNTERM_PROFILE:-}" ]; then
        command gh "$@"
        return $?
    fi
    case "$*" in
        "repo delete"*|"repo archive"*)
            _unterm_guard_confirm "gh $1 $2" "gh $*" || return 1
            ;;
    esac
    command gh "$@"
}

# --- aws: bucket deletes, recursive removal ---
aws() {
    if [ -z "${UNTERM_PROFILE:-}" ]; then
        command aws "$@"
        return $?
    fi
    local joined="$*"
    case "$joined" in
        *"s3 rb"*|*"s3api delete-bucket"*)
            _unterm_guard_confirm "aws bucket delete" "aws $joined" || return 1
            ;;
        *"s3 rm"*"--recursive"*|*"s3 rm"*" -r "*)
            _unterm_guard_confirm "aws recursive rm" "aws $joined" || return 1
            ;;
    esac
    command aws "$@"
}

# --- npm: unpublish ---
npm() {
    if [ -z "${UNTERM_PROFILE:-}" ]; then
        command npm "$@"
        return $?
    fi
    case "$*" in
        "unpublish"*)
            _unterm_guard_confirm "npm unpublish" "npm $*" || return 1
            ;;
    esac
    command npm "$@"
}

# --- git: --force / -f push to non-fork remote ---
# (We don't try to detect "is this a fork" here — too platform-specific.
# Force-pushes to ANY remote get a prompt. --force-with-lease is exempt
# because it's the safe variant.)
git() {
    if [ -z "${UNTERM_PROFILE:-}" ]; then
        command git "$@"
        return $?
    fi
    local args="$*"
    case "$args" in
        push*--force-with-lease*)
            : # safe variant, allow through
            ;;
        push*--force*|"push -f"*|push*" -f "*|push*" -f")
            _unterm_guard_confirm "git push --force" "git $args" || return 1
            ;;
    esac
    command git "$@"
}

# --- vercel: rm / remove ---
vercel() {
    if [ -z "${UNTERM_PROFILE:-}" ]; then
        command vercel "$@"
        return $?
    fi
    case "$*" in
        "rm"*|"remove"*)
            _unterm_guard_confirm "vercel rm" "vercel $*" || return 1
            ;;
    esac
    command vercel "$@"
}
"##;

/// Fish shell variant. Fish uses different function syntax and `string
/// match` for pattern matching, so the script is rewritten rather than
/// reused. Same pattern coverage as the bash/zsh script.
pub const FISH_SCRIPT: &str = r##"# Unterm destructive-command guard (v0.13, fish)
#
# Source from ~/.config/fish/config.fish:
#
#   if test -n "$UNTERM_PROFILE"
#       unterm-cli profile shell-integration fish | source
#   end

function _unterm_guard_confirm
    set -l label $argv[1]
    set -l full $argv[2]
    printf '\n\033[1;33m[Unterm guard]\033[0m profile=%s\n' "$UNTERM_PROFILE"
    printf '\033[1;33m[Unterm guard]\033[0m About to run: %s\n' "$full"
    printf '\033[1;33m[Unterm guard]\033[0m %s — proceed? [y/N] ' "$label"
    read -P "" -l reply
    switch $reply
        case y Y yes YES
            return 0
        case '*'
            printf '\033[1;33m[Unterm guard]\033[0m Aborted.\n'
            return 1
    end
end

function gh
    if test -z "$UNTERM_PROFILE"
        command gh $argv
        return $status
    end
    if string match -q -- "repo delete*" "$argv" \
       || string match -q -- "repo archive*" "$argv"
        _unterm_guard_confirm "gh $argv[1] $argv[2]" "gh $argv" || return 1
    end
    command gh $argv
end

function aws
    if test -z "$UNTERM_PROFILE"
        command aws $argv
        return $status
    end
    set -l joined (string join " " -- $argv)
    if string match -q -- "*s3 rb*" "$joined" \
       || string match -q -- "*s3api delete-bucket*" "$joined"
        _unterm_guard_confirm "aws bucket delete" "aws $joined" || return 1
    else if string match -q -- "*s3 rm*--recursive*" "$joined"
        _unterm_guard_confirm "aws recursive rm" "aws $joined" || return 1
    end
    command aws $argv
end

function npm
    if test -z "$UNTERM_PROFILE"
        command npm $argv
        return $status
    end
    if string match -q -- "unpublish*" "$argv"
        _unterm_guard_confirm "npm unpublish" "npm $argv" || return 1
    end
    command npm $argv
end

function git
    if test -z "$UNTERM_PROFILE"
        command git $argv
        return $status
    end
    set -l joined (string join " " -- $argv)
    if string match -q -- "push*--force-with-lease*" "$joined"
        # safe variant
    else if string match -q -- "push*--force*" "$joined" \
            || string match -q -- "push*-f*" "$joined"
        _unterm_guard_confirm "git push --force" "git $joined" || return 1
    end
    command git $argv
end

function vercel
    if test -z "$UNTERM_PROFILE"
        command vercel $argv
        return $status
    end
    if string match -q -- "rm*" "$argv" || string match -q -- "remove*" "$argv"
        _unterm_guard_confirm "vercel rm" "vercel $argv" || return 1
    end
    command vercel $argv
end
"##;

/// Return the guard script for the named shell. Supported shells:
/// `bash`, `zsh`, `fish`.
pub fn script_for(shell: &str) -> Result<&'static str> {
    match shell {
        "bash" | "zsh" => Ok(BASH_ZSH_SCRIPT),
        "fish" => Ok(FISH_SCRIPT),
        other => Err(anyhow!(
            "unsupported shell: {other}. Supported: bash, zsh, fish."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bash_zsh_script_covers_known_destructive_patterns() {
        let s = script_for("bash").unwrap();
        // Sanity: all the binaries we claim to wrap appear in the script
        for needle in &["gh()", "aws()", "npm()", "git()", "vercel()"] {
            assert!(s.contains(needle), "missing wrapper for {needle}");
        }
        // Sanity: each known destructive case is matched
        for needle in &[
            "repo delete",
            "repo archive",
            "s3 rb",
            "s3api delete-bucket",
            "s3 rm",
            "unpublish",
            "--force",
            "--force-with-lease",
            "rm",
        ] {
            assert!(s.contains(needle), "destructive pattern missing: {needle}");
        }
        // The UNTERM_PROFILE bypass must be honored
        assert!(s.contains("UNTERM_PROFILE"));
    }

    #[test]
    fn fish_script_emits_function_definitions() {
        let s = script_for("fish").unwrap();
        for needle in &["function gh", "function aws", "function npm", "function git"] {
            assert!(s.contains(needle), "fish wrapper missing: {needle}");
        }
    }

    #[test]
    fn unsupported_shell_errors_with_useful_message() {
        let err = script_for("powershell").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("unsupported shell"));
        assert!(msg.contains("bash, zsh, fish"));
    }
}
