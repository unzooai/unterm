//! `scope.*`, `artifact.*`, `audit.verify`, `task.*_evidence` — what happened
//! and where it was allowed to happen.
//!
//! Four namespaces in one module because they are one story told at four
//! points: a workspace bounds the work, artifacts are what it produced, the
//! audit chain is the record of it, and an evidence bundle is all of that
//! handed to somebody who was not there.
//!
//! On the name `scope.*`: this surface has meant saved pane layouts by
//! `workspace.*` since long before M6, and renaming that would break every
//! agent in the field for the sake of a word. So the filesystem scopes get
//! their own namespace and the old one keeps its meaning.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use unterm_services::path_scope::PathAccess;
use unterm_services::{artifacts, audit_store, evidence, workspace_scope};

pub const METHODS: &[&str] = &[
    "scope.list",
    "scope.create",
    "scope.check",
    "scope.archive",
    "artifact.list",
    "artifact.usage",
    "artifact.verify",
    "artifact.forget",
    "audit.verify",
    "task.export_evidence",
    "task.verify_evidence",
];

pub fn handles(method: &str) -> bool {
    METHODS.contains(&method)
}

fn text(params: &Value, key: &str) -> Result<String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("Missing '{key}'"))
}

pub fn dispatch(method: &str, params: &Value) -> Result<Value> {
    match method {
        "scope.list" => Ok(json!({"workspaces": workspace_scope::list()?})),

        "scope.create" => {
            let workspace = workspace_scope::create(&text(params, "name")?, text(params, "path")?)?;
            Ok(json!({"workspace": workspace}))
        }

        "scope.check" => {
            let access = match params.get("access").and_then(Value::as_str).unwrap_or("read") {
                "write" => PathAccess::Write,
                "read" => PathAccess::Read,
                other => {
                    return Err(anyhow!(
                        "{other:?} is not an access; expected read or write"
                    ))
                }
            };
            let decision = workspace_scope::check(
                &text(params, "workspace")?,
                access,
                text(params, "path")?,
            )?;
            Ok(serde_json::to_value(decision)?)
        }

        "scope.archive" => {
            let id = text(params, "workspace")?;
            Ok(json!({"workspace": id, "archived": workspace_scope::archive(&id)?}))
        }

        "artifact.list" => {
            let store = unterm_services::cockpit::fleet_store::tasks()
                .ok_or_else(|| anyhow!("there is no task store"))?;
            let list = match params.get("task_id").and_then(Value::as_str) {
                Some(task) => store.artifacts_for_task(task)?,
                None => store.artifacts()?,
            };
            Ok(json!({"artifacts": list}))
        }

        "artifact.usage" => Ok(json!({"usage": artifacts::usage()?})),

        "artifact.verify" => {
            let store = unterm_services::cockpit::fleet_store::tasks()
                .ok_or_else(|| anyhow!("there is no task store"))?;
            let id = text(params, "artifact")?;
            let artifact = store
                .artifact(&id)?
                .ok_or_else(|| anyhow!("no such artifact: {id}"))?;
            Ok(json!({
                "artifact": id,
                "sha256": artifact.sha256,
                "intact": artifacts::verify(&artifact)?,
            }))
        }

        "artifact.forget" => {
            let id = text(params, "artifact")?;
            Ok(json!({"artifact": id, "forgotten": artifacts::forget(&id)?}))
        }

        "audit.verify" => Ok(serde_json::to_value(audit_store::verify_chain())?),

        "task.export_evidence" => {
            let manifest = evidence::export(&text(params, "task_id")?, text(params, "path")?)?;
            Ok(json!({"manifest": manifest}))
        }

        "task.verify_evidence" => Ok(serde_json::to_value(evidence::verify(text(
            params, "path",
        )?)?)?),

        other => Err(anyhow!("records dispatch reached {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_method_is_dispatched() {
        // The handler's drift check reads this table rather than scanning for
        // literal match arms, so a name here with no arm would look published
        // and answer "unknown method".
        for method in METHODS {
            assert!(handles(method));
            if let Err(error) = dispatch(method, &json!({})) {
                assert!(
                    !error.to_string().contains("records dispatch reached"),
                    "{method} has no arm"
                );
            }
        }
    }

    #[test]
    fn an_access_nobody_defined_is_refused_by_name() {
        let error = dispatch(
            "scope.check",
            &json!({"workspace": "wsp_1", "path": "/tmp", "access": "execute"}),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("execute"), "{error}");
        assert!(error.contains("read or write"), "{error}");
    }

    #[test]
    fn exporting_needs_somewhere_to_put_it() {
        let error = dispatch("task.export_evidence", &json!({"task_id": "tsk_1"}))
            .unwrap_err()
            .to_string();
        assert!(error.contains("path"), "{error}");
    }
}
