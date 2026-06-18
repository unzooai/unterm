//! Template expansion for paths and shell-cmd args in agent manifests.
//!
//! Manifests can't hardcode absolute paths since the user's $HOME varies
//! and per-profile sandbox dirs are computed at runtime. The supported
//! placeholders are intentionally small — anything fancier should be
//! handled by the agent itself, not by string interpolation here.
//!
//! Placeholders:
//!   {{HOME}}         — user's real home dir
//!   {{PROFILE_HOME}} — Unterm's sandbox home for (profile, agent),
//!                      see paths::agent_home()
//!   {{CWD}}          — current working directory at launch (only set
//!                      during launch arg expansion; otherwise empty)
//!   {{PROFILE_ID}}   — slugified identity profile id
//!   {{AGENT_ID}}     — manifest id

use crate::errors::{AgentError, Result};
use crate::paths;
use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct TemplateCtx {
    pub profile_id: String,
    pub agent_id: String,
    pub cwd: Option<String>,
    /// Absolute path to the unterm-cli binary (used to wire agents at the
    /// `unterm-cli mcp-stdio` bridge). Empty when MCP auto-wire is off.
    pub unterm_cli: Option<String>,
    /// MCP control-server connection details for the current instance.
    pub mcp_host: Option<String>,
    pub mcp_port: Option<u16>,
    pub mcp_token: Option<String>,
}

impl TemplateCtx {
    pub fn variables(&self) -> Result<HashMap<&'static str, String>> {
        let mut vars = HashMap::new();
        let home =
            dirs_next::home_dir().ok_or_else(|| AgentError::ParseFailed("$HOME not set".into()))?;
        vars.insert("HOME", home.to_string_lossy().to_string());
        vars.insert(
            "PROFILE_HOME",
            paths::agent_home(&self.profile_id, &self.agent_id)?
                .to_string_lossy()
                .to_string(),
        );
        vars.insert("PROFILE_ID", self.profile_id.clone());
        vars.insert("AGENT_ID", self.agent_id.clone());
        vars.insert("CWD", self.cwd.clone().unwrap_or_default());
        vars.insert("UNTERM_CLI", self.unterm_cli.clone().unwrap_or_default());
        vars.insert("MCP_HOST", self.mcp_host.clone().unwrap_or_default());
        vars.insert(
            "MCP_PORT",
            self.mcp_port.map(|p| p.to_string()).unwrap_or_default(),
        );
        vars.insert("MCP_TOKEN", self.mcp_token.clone().unwrap_or_default());
        Ok(vars)
    }
}

pub fn expand(template: &str, ctx: &TemplateCtx) -> Result<String> {
    let vars = ctx.variables()?;
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after.find("}}").ok_or_else(|| {
            AgentError::ParseFailed(format!("unterminated template in {template:?}"))
        })?;
        let name = after[..end].trim();
        let replacement = vars.get(name).cloned().unwrap_or_else(|| {
            log::warn!("unknown template var {{{{{name}}}}} — left as empty string");
            String::new()
        });
        out.push_str(&replacement);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

pub fn expand_args(args: &[String], ctx: &TemplateCtx) -> Result<Vec<String>> {
    args.iter().map(|a| expand(a, ctx)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_basic() {
        let ctx = TemplateCtx {
            profile_id: "work".into(),
            agent_id: "claude-code".into(),
            cwd: Some("/tmp/proj".into()),
            ..Default::default()
        };
        let out = expand("agent={{AGENT_ID}} cwd={{CWD}}", &ctx).unwrap();
        assert_eq!(out, "agent=claude-code cwd=/tmp/proj");
    }

    #[test]
    fn unknown_var_is_empty() {
        let ctx = TemplateCtx::default();
        let out = expand("x={{NOPE}}y", &ctx).unwrap();
        assert_eq!(out, "x=y");
    }

    #[test]
    fn unterminated_is_error() {
        let ctx = TemplateCtx::default();
        assert!(expand("x={{HOME", &ctx).is_err());
    }
}
