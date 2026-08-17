//! What an agent written against an older Unterm may still call.
//!
//! M1 moved the MCP server out of the GUI and into `unterm-core`. Nothing
//! about that migration is supposed to be visible to a client, and the
//! existing count contract in `meta.rs` cannot see the failure that would
//! matter most: a method renamed keeps the count identical and breaks every
//! agent that called it. This is the other half — the names themselves,
//! frozen.
//!
//! Adding methods is fine and deliberately does not fail here; the count
//! contract already forces a new one to be acknowledged. Removing or
//! renaming one is what this catches, because the agents in the wild are not
//! recompiled when we move code between processes.

/// Every method name the surface published at 0.66.0, when the Core became
/// the only server. Do not delete a line to make a build pass: a name that
/// leaves this list is a client that breaks in the field.
const FROZEN_METHODS: &[&str] = &[
    "agent.identify",
    "agent.list_trusted",
    "agent.signal",
    "agent.status",
    "agent.trust",
    "agent.untrust",
    "agent.whoami",
    "capture.clipboard",
    "capture.screen",
    "capture.scrollback",
    "capture.select",
    "capture.window",
    "capture.window_scroll",
    "cockpit.inbox",
    "exec.cancel",
    "exec.run",
    "exec.run_wait",
    "exec.send",
    "exec.status",
    "fleet.clean",
    "fleet.launch",
    "fleet.list",
    "fleet.retry",
    "ghost.debug",
    "instance.close",
    "instance.focus",
    "instance.info",
    "instance.lifecycle",
    "instance.list",
    "instance.set_title",
    "meta.surface",
    "orchestrate.broadcast",
    "orchestrate.launch",
    "orchestrate.wait",
    "policy.check",
    "policy.set",
    "profile.audit",
    "profile.current",
    "profile.list",
    "proxy.clash_select",
    "proxy.clash_set_controller",
    "proxy.clash_status",
    "proxy.configure",
    "proxy.disable",
    "proxy.env",
    "proxy.nodes",
    "proxy.rotation",
    "proxy.set_nodes",
    "proxy.speedtest",
    "proxy.status",
    "proxy.switch",
    "review.diff",
    "review.discard",
    "review.list",
    "review.merge",
    "review.rollback",
    "review.verify",
    "screen.clear",
    "screen.cursor",
    "screen.detect_errors",
    "screen.read",
    "screen.scroll",
    "screen.scrollback_text",
    "screen.search",
    "screen.text",
    "selftest.run",
    "server.capabilities",
    "server.health",
    "server.info",
    "session.audit_log",
    "session.create",
    "session.cwd",
    "session.destroy",
    "session.env",
    "session.export_markdown",
    "session.focus",
    "session.get",
    "session.history",
    "session.idle",
    "session.input",
    "session.list",
    "session.paste",
    "session.recording_attach_trace",
    "session.recording_list",
    "session.recording_read",
    "session.recording_start",
    "session.recording_status",
    "session.recording_stop",
    "session.resize",
    "session.set_env",
    "session.split",
    "session.status",
    "session.suggest",
    "session.suggest_cancel",
    "session.suggest_list",
    "session.suggest_status",
    "signal.send",
    "system.info",
    "system.launch_admin",
    "upload.file",
    "workspace.list",
    "workspace.restore",
    "workspace.save",
];

fn published_methods() -> Vec<String> {
    let surface = unterm_mcp::meta::surface(&serde_json::json!({})).expect("meta.surface");
    surface["mcp_methods"]
        .as_array()
        .expect("mcp_methods is an array")
        .iter()
        .map(|method| {
            method
                .get("name")
                .and_then(|name| name.as_str())
                .or_else(|| method.as_str())
                .unwrap_or_default()
                .to_string()
        })
        .filter(|name| !name.is_empty())
        .collect()
}

#[test]
fn every_frozen_method_is_still_published() {
    let published = published_methods();
    let missing: Vec<&&str> = FROZEN_METHODS
        .iter()
        .filter(|frozen| !published.iter().any(|name| name == *frozen))
        .collect();
    assert!(
        missing.is_empty(),
        "these methods left the surface; any agent still calling them now gets \
         an unknown-method error: {missing:?}"
    );
}

#[test]
fn the_frozen_list_is_sorted_and_unique() {
    // So a merge that adds a name cannot quietly shadow one already there,
    // and a reviewer can diff two versions of the list by eye.
    let mut sorted = FROZEN_METHODS.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        FROZEN_METHODS.len(),
        "the frozen list has duplicates"
    );
    assert_eq!(
        sorted.as_slice(),
        FROZEN_METHODS,
        "the frozen list is not in sorted order"
    );
}

#[test]
fn the_surface_publishes_no_method_twice() {
    let published = published_methods();
    let mut unique = published.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(
        unique.len(),
        published.len(),
        "a method name is published twice; a client cannot tell which one it reaches"
    );
}

/// Every published method must have a risk classification.
///
/// The gateway refuses what it cannot classify, on the reasoning that an
/// unregistered action is exactly the one that will surprise somebody. That
/// is only safe if the registry actually covers the surface — otherwise the
/// first caller of a forgotten method gets a refusal instead of a terminal.
/// This is the check that keeps the two in step, and it fails at build time
/// rather than in front of a user.
#[test]
fn every_published_method_has_a_risk_classification() {
    let unclassified: Vec<&&str> = FROZEN_METHODS
        .iter()
        .filter(|method| unterm_gateway::risk_of(method).is_none())
        .collect();
    assert!(
        unclassified.is_empty(),
        "these methods have no entry in the gateway's risk registry, so the \
         gateway would refuse them as unclassified: {unclassified:#?}"
    );
}

/// Every method the surface publishes *today* must be classified.
///
/// The frozen list is the 0.66.0 contract and does not grow with each
/// release, which means checking only it would stop covering the surface the
/// moment anything new shipped. The gateway refuses what it cannot classify,
/// so a method added without an entry is one whose first caller gets a
/// refusal instead of a terminal — and nothing would have caught it.
#[test]
fn every_method_published_now_has_a_risk_classification() {
    let unclassified: Vec<String> = published_methods()
        .into_iter()
        .filter(|method| unterm_gateway::risk_of(method).is_none())
        .collect();
    assert!(
        unclassified.is_empty(),
        "these are published but unclassified, so the gateway would refuse them: {unclassified:#?}"
    );
}

/// No door may keep its own copy of the rules.
///
/// M3's whole claim is that every entry point reaches the same verdict,
/// which is only true while there is one implementation to reach. A second
/// risk table or policy evaluator growing back inside a handler is exactly
/// how that claim quietly stops being true — the copy starts identical and
/// drifts, and the drift is invisible until an action is allowed at one door
/// and refused at another.
///
/// This is a source check rather than a behavioural one on purpose: by the
/// time a divergence is observable in behaviour, it has already shipped.
#[test]
fn no_handler_keeps_a_private_risk_table_or_policy_evaluator() {
    let handler = include_str!("../src/handler.rs");
    // Names of the copies that used to live there. Each was deleted when the
    // door was routed through `unterm_services::gateway`; a reappearance
    // means somebody rebuilt the thing this milestone removed.
    for banned in [
        "enum ActionRisk",
        "fn action_risk(",
        "struct ActionGatewayDecision",
        "fn gateway_decision_for_command(",
        // The policy evaluator itself, which outlived the decision struct by
        // one commit and would have drifted just as quietly.
        "enum PolicyVerdict",
        "fn policy_verdict(",
    ] {
        assert!(
            !handler.contains(banned),
            "`{banned}` is back in the MCP handler. Decisions belong to \
             unterm-gateway, asked through unterm_services::gateway, so that \
             every door reaches the same answer — add the case there instead."
        );
    }
}

/// The doors that make decisions ask the shared gateway.
///
/// Weaker than proving no bypass exists — that needs a call-graph — but it
/// catches the realistic regression: a handler that decides for itself
/// without mentioning the gateway at all.
#[test]
fn the_deciding_doors_reference_the_shared_gateway() {
    let handler = include_str!("../src/handler.rs");
    assert!(
        handler.contains("unterm_services::gateway::admit"),
        "the MCP handler no longer calls the shared gateway; if the call moved, \
         point this test at its new home rather than deleting it"
    );
    // The PTY write path specifically: the door with no protocol of its own,
    // and the one whose private rules would be least visible.
    let pty_gate = handler
        .split("fn gate_pty_write")
        .nth(1)
        .expect("gate_pty_write should still exist");
    let body = &pty_gate[..pty_gate.len().min(4000)];
    assert!(
        body.contains("unterm_services::gateway::admit"),
        "the PTY write gate stopped asking the shared gateway"
    );
}
