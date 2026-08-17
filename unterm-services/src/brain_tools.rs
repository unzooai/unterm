//! Where a model's tool request meets the gateway.
//!
//! A brain adapter produces a *request*: the model would like a shell run, a
//! file written, a pane killed. Nothing in `unterm-brain` can perform one —
//! there is no gateway there, and no code that would call it. This is the
//! only bridge, and it goes through the same door as everything else.
//!
//! The translation is the interesting part. A model says `Bash` or `shell` or
//! `run_command`; the gateway judges `exec.run`. Naming a tool the gateway
//! does not know must not become permission, so an unrecognised tool is
//! judged as a mutation and asked about, never waved through. That asymmetry
//! is deliberate: the cost of asking about something harmless is a prompt,
//! and the cost of the other mistake is whatever the tool did.

use crate::gateway::{admit, Outcome, Passage, SettingsPolicy};
use serde_json::Value;
use unterm_brain::BrainEvent;
use unterm_gateway::{ActionContext, Entry, PolicySource};

/// Who is asking, and what for.
#[derive(Clone, Debug, Default)]
pub struct Caller {
    /// The agent's name, for the audit trail and for actor-scoped grants.
    pub actor: Option<String>,
    /// The task this turn belongs to, so one approval can cover it.
    pub task_id: Option<String>,
}

/// The gateway method a tool maps to, and the command when there is one.
///
/// Returned rather than applied so the caller can log the translation: an
/// audit line saying "Bash → exec.run" is the one that makes an approval
/// prompt legible months later.
#[derive(Clone, Debug, PartialEq)]
pub struct Mapped {
    pub method: String,
    pub command: Option<String>,
    pub resource: Option<String>,
}

/// Translate a tool call into the vocabulary every door shares.
pub fn map_tool(name: &str, arguments: &Value) -> Mapped {
    let lowered = name.to_ascii_lowercase();
    let text = |keys: &[&str]| -> Option<String> {
        keys.iter()
            .find_map(|key| arguments.get(*key).and_then(Value::as_str))
            .map(str::to_string)
    };

    let (method, command, resource) = match lowered.as_str() {
        "bash" | "shell" | "exec" | "run_command" | "local_shell" | "terminal" => (
            "exec.run",
            text(&["command", "cmd", "script"]),
            None,
        ),
        "read" | "read_file" | "cat" | "view" => {
            ("brain.read", None, text(&["path", "file", "file_path"]))
        }
        "write" | "write_file" | "edit" | "edit_file" | "apply_patch" | "str_replace" => (
            "brain.write",
            None,
            text(&["path", "file", "file_path"]),
        ),
        "glob" | "grep" | "search" | "list_dir" | "ls" => {
            ("brain.list", None, text(&["path", "pattern"]))
        }
        "webfetch" | "web_search" | "fetch" => ("brain.fetch", None, text(&["url"])),
        _ => (
            // Unknown tools are mutations. The gateway's own classification
            // treats an unrecognised method as writing rather than reading,
            // and a bridge that guessed "probably harmless" would be the hole
            // every future tool walks through.
            "brain.tool",
            text(&["command", "cmd"]),
            text(&["path", "file", "file_path", "url"]),
        ),
    };
    Mapped {
        method: method.to_string(),
        command,
        resource,
    }
}

/// Run one tool request past the gateway.
///
/// Returns `None` when the event is not a tool request — a caller that pipes
/// its whole event stream through here should get nothing for the rest, not a
/// verdict about a sentence the model said.
pub fn admit_tool(
    event: &BrainEvent,
    caller: &Caller,
    policy: &dyn PolicySource,
) -> Option<Passage> {
    let BrainEvent::ToolRequested {
        name, arguments, ..
    } = event
    else {
        return None;
    };
    let mapped = map_tool(name, arguments);
    let mut context = ActionContext::new(mapped.method)
        // Recorded, not judged on: the verdict is the same through every
        // door, but an audit trail that cannot say a model asked is missing
        // the thing people want to know afterwards.
        .entry(Entry::Brain);
    context.command = mapped.command;
    context.resource = mapped.resource;
    context.actor = caller.actor.clone();
    context.task_id = caller.task_id.clone();
    Some(admit(&context, policy))
}

/// Whether the caller may go ahead and run the tool now.
pub fn may_proceed(passage: &Passage) -> bool {
    matches!(passage.outcome, Outcome::Proceed { .. })
}

/// The policy a brain runs under when the caller has not configured one.
pub fn default_policy() -> SettingsPolicy {
    SettingsPolicy::off()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cockpit::fleet_store;
    use serde_json::json;
    use unterm_gateway::{Code, Risk};

    fn isolate() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("UNTERM_TASKS_DB", dir.path().join("tasks.db"));
        fleet_store::reset_for_tests();
        dir
    }

    fn request(name: &str, arguments: Value) -> BrainEvent {
        BrainEvent::ToolRequested {
            call_id: "c1".into(),
            name: name.into(),
            arguments,
        }
    }

    #[test]
    fn the_shell_a_model_asks_for_is_the_shell_the_policy_judges() {
        let _dir = isolate();
        let policy = SettingsPolicy::new(true, vec!["rm -rf".to_string()], Vec::new());
        // Every CLI has its own name for the same tool; all of them must land
        // on the command the user's blocklist is written against.
        for name in ["Bash", "shell", "local_shell", "run_command"] {
            let passage = admit_tool(
                &request(name, json!({"command": "rm -rf /"})),
                &Caller::default(),
                &policy,
            )
            .expect("a tool request");
            assert_eq!(
                passage.outcome,
                Outcome::Refuse,
                "{name} escaped the blocklist"
            );
            assert_eq!(passage.verdict.code, Code::PolicyBlockedPattern);
        }
    }

    #[test]
    fn a_tool_nobody_has_heard_of_is_not_thereby_safe() {
        // The hole this closes: a new tool name appearing in a CLI update and
        // being waved through because the bridge did not recognise it.
        let _dir = isolate();
        let passage = admit_tool(
            &request("some_new_tool", json!({"path": "/etc/hosts"})),
            &Caller::default(),
            &SettingsPolicy::off(),
        )
        .unwrap();
        assert_ne!(passage.verdict.risk, Risk::Readonly);
    }

    #[test]
    fn reading_a_file_does_not_interrupt_anybody() {
        let _dir = isolate();
        let passage = admit_tool(
            &request("Read", json!({"file_path": "/tmp/x"})),
            &Caller::default(),
            &SettingsPolicy::off(),
        )
        .unwrap();
        assert_eq!(passage.verdict.risk, Risk::Readonly);
        assert!(may_proceed(&passage));
        assert!(fleet_store::tasks()
            .unwrap()
            .pending_approvals()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn what_the_model_said_is_not_a_tool_request() {
        let _dir = isolate();
        for event in [
            BrainEvent::Text { text: "rm -rf /".into() },
            BrainEvent::Reasoning {
                text: "I could rm -rf /".into(),
            },
            BrainEvent::ToolResult {
                call_id: "c1".into(),
                ok: true,
                output: Some("done".into()),
            },
        ] {
            assert!(
                admit_tool(&event, &Caller::default(), &SettingsPolicy::off()).is_none(),
                "a {} was judged as if it were a tool call",
                event.kind()
            );
        }
    }

    #[test]
    fn the_task_carries_through_so_one_answer_can_cover_a_turn() {
        let _dir = isolate();
        let caller = Caller {
            actor: Some("codex".into()),
            task_id: Some("tsk_1".into()),
        };
        let passage = admit_tool(
            &request("Bash", json!({"command": "git push"})),
            &caller,
            &SettingsPolicy::off(),
        )
        .unwrap();
        // Whatever the verdict, the audit trail knows who asked and what for.
        assert!(passage.verdict.risk >= Risk::LocalMutation);
        let pending = fleet_store::tasks().unwrap().pending_approvals().unwrap();
        for approval in pending {
            assert_eq!(approval.actor.as_deref(), Some("codex"));
            assert_eq!(approval.task_id.as_deref(), Some("tsk_1"));
        }
    }

    #[test]
    fn the_brain_crate_cannot_run_a_tool_even_if_someone_wanted_it_to() {
        // The property M4-02 rests on, checked structurally because it is the
        // kind that decays silently: adapters are parsers, and the only place
        // in the brain crate that starts a process is the one that starts the
        // brain itself.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("unterm-brain/src");
        let mut offenders = Vec::new();
        for entry in std::fs::read_dir(&root).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let source = std::fs::read_to_string(&path).unwrap();
            if name != "runtime.rs" && source.contains("Command::new") {
                offenders.push(name);
            }
        }
        assert!(
            offenders.is_empty(),
            "these started a process outside the brain's own launcher: {offenders:?}"
        );
    }
}
