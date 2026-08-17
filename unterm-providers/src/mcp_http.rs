//! Talking to a provider that speaks MCP over HTTP.
//!
//! One client, two users: the Unzoo binding, and any provider that drops a
//! descriptor saying where it is and which of its tools belong to which
//! capability. Keeping it generic is what makes a descriptor worth writing —
//! a provider nobody at Unterm has heard of can still be leased, audited and
//! revoked exactly like the one that was built in.
//!
//! The families table is required, not optional. A provider that does not say
//! which capability a tool belongs to gets no capability at all: an unmapped
//! tool is refused rather than covered by whichever lease is nearest.

use crate::{Call, Capability, Endpoint, Failure, Handshake, Identity, Provider, ProviderManifest};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

/// How a provider's tool names map to capabilities.
#[derive(Clone, Debug, PartialEq)]
pub enum Families {
    /// `browser_* -> Browser`, keyed by the part before the first underscore.
    Prefixes(BTreeMap<String, Capability>),
}

impl Families {
    pub fn of(&self, tool: &str) -> Option<Capability> {
        match self {
            Families::Prefixes(table) => {
                table.get(tool.split('_').next().unwrap_or_default()).copied()
            }
        }
    }
}

/// A provider reached over HTTP.
pub struct HttpMcpProvider {
    id: String,
    families: Families,
    endpoint: String,
    client: reqwest::blocking::Client,
    next_id: AtomicI64,
}

impl HttpMcpProvider {
    pub fn new(id: impl Into<String>, endpoint: impl Into<String>, families: Families) -> Self {
        Self {
            id: id.into(),
            families,
            endpoint: endpoint.into(),
            client: reqwest::blocking::Client::builder()
                // A browser action can legitimately take a while; a handshake
                // cannot, but the same client serves both and the shorter
                // timeout would break real work.
                .timeout(Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            next_id: AtomicI64::new(1),
        }
    }

    /// From whatever discovery found.
    pub fn from_manifest(manifest: &ProviderManifest) -> Result<Self, Failure> {
        match &manifest.endpoint {
            Endpoint::Http { url } => Ok(Self::new(
                manifest.id.clone(),
                url,
                Families::Prefixes(manifest.families.clone()),
            )),
            other => Err(Failure::Incompatible(format!(
                "this build speaks HTTP to providers; the manifest says {other:?}"
            ))),
        }
    }

    fn rpc(&self, method: &str, params: Value) -> Result<Value, Failure> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let response = self
            .client
            .post(&self.endpoint)
            .header("content-type", "application/json")
            // The service answers either shape; asking for both is what the
            // MCP HTTP transport expects a client to do.
            .header("accept", "application/json, text/event-stream")
            .json(&body)
            .send()
            .map_err(|error| Failure::Offline(error.to_string()))?;

        if !response.status().is_success() {
            return Err(Failure::Provider(format!(
                "{} answered {}",
                self.endpoint,
                response.status()
            )));
        }
        let text = response
            .text()
            .map_err(|error| Failure::Provider(error.to_string()))?;
        let value = parse_body(&text)?;
        if let Some(error) = value.get("error") {
            return Err(Failure::Provider(
                error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or(&error.to_string())
                    .to_string(),
            ));
        }
        Ok(value.get("result").cloned().unwrap_or(Value::Null))
    }
}

/// Read either a JSON body or an SSE frame carrying one.
fn parse_body(text: &str) -> Result<Value, Failure> {
    let trimmed = text.trim_start();
    if trimmed.starts_with('{') {
        // `strict(false)`: the service's own instructions field contains raw
        // control characters, and refusing the whole handshake over a stray
        // newline in prose would be a strange place to be strict.
        return lenient_json(trimmed);
    }
    for line in text.lines() {
        if let Some(payload) = line.strip_prefix("data:") {
            let payload = payload.trim();
            if !payload.is_empty() && payload != "[DONE]" {
                return lenient_json(payload);
            }
        }
    }
    Err(Failure::Provider(format!(
        "could not read the provider's answer: {}",
        text.chars().take(120).collect::<String>()
    )))
}

fn lenient_json(text: &str) -> Result<Value, Failure> {
    match serde_json::from_str(text) {
        Ok(value) => Ok(value),
        Err(_) => {
            // Escape the control characters that a provider put in a string
            // and try once more, rather than losing a whole handshake to
            // somebody's multi-line description field.
            let repaired: String = text
                .chars()
                .map(|c| match c {
                    '\n' => "\\n".to_string(),
                    '\r' => "\\r".to_string(),
                    '\t' => "\\t".to_string(),
                    other => other.to_string(),
                })
                .collect();
            serde_json::from_str(&repaired).map_err(|error| {
                Failure::Provider(format!("the provider's answer was not JSON: {error}"))
            })
        }
    }
}

impl Provider for HttpMcpProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn handshake(&self) -> Result<Handshake, Failure> {
        let result = self.rpc(
            "initialize",
            json!({
                "protocolVersion": crate::negotiate::preferred(crate::discovery::PROTOCOLS)
                    .unwrap_or_default(),
                "capabilities": {},
                // Unterm says who it is. Half of "mutual": the provider can
                // refuse or scope by client, and its own logs can say who
                // drove it.
                "clientInfo": {"name": "unterm", "version": env!("CARGO_PKG_VERSION")},
            }),
        )?;

        let protocol = result
            .get("protocolVersion")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let protocol = crate::negotiate::settle(crate::discovery::PROTOCOLS, protocol)
            .map_err(Failure::Incompatible)?;

        let info = result.get("serverInfo").cloned().unwrap_or(Value::Null);
        let identity = Identity {
            name: info
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            version: info
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
        };

        // What it can actually do, from what it actually offers.
        let tools = self.rpc("tools/list", json!({}))?;
        let mut capabilities: Vec<Capability> = tools
            .get("tools")
            .and_then(Value::as_array)
            .map(|tools| {
                tools
                    .iter()
                    .filter_map(|tool| tool.get("name").and_then(Value::as_str))
                            .filter_map(|name| self.families.of(name))
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect()
            })
            .unwrap_or_default();
        capabilities.sort();

        Ok(Handshake {
            identity,
            protocol,
            capabilities,
        })
    }

    fn call(&self, call: &Call) -> Result<Value, Failure> {
        // The capability on the lease must match the tool being called, or a
        // browser lease would reach the cookie jar.
        match self.families.of(&call.method) {
            Some(family) if family == call.capability => {}
            Some(family) => return Err(Failure::Unsupported(family)),
            None => {
                return Err(Failure::Provider(format!(
                    "{} belongs to no capability this provider declared, so no lease can cover it",
                    call.method
                )))
            }
        }
        let result = self.rpc(
            "tools/call",
            json!({"name": call.method, "arguments": call.params}),
        )?;
        if result
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return Err(Failure::Provider(
                result
                    .get("content")
                    .map(|content| content.to_string())
                    .unwrap_or_else(|| "the provider reported an error".to_string()),
            ));
        }
        Ok(result)
    }

    fn cancel(&self, call_id: &str) -> Result<(), Failure> {
        // MCP's cancellation is a notification: no answer comes back, so the
        // most this can honestly report is that it was delivered.
        self.rpc(
            "notifications/cancelled",
            json!({"requestId": call_id, "reason": "unterm cancelled the task"}),
        )
        .map(|_| ())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Families {
        Families::Prefixes(crate::unzoo::FAMILIES.iter().map(|(prefix, capability)| {
            (prefix.to_string(), *capability)
        }).collect())
    }

    #[test]
    fn an_unmapped_tool_belongs_to_no_capability() {
        // The rule that keeps a provider update from quietly widening what a
        // lease covers.
        assert_eq!(table().of("quantum_teleport"), None);
        assert_eq!(table().of(""), None);
    }

    #[test]
    fn a_provider_that_declared_nothing_can_do_nothing() {
        let empty = Families::Prefixes(BTreeMap::new());
        assert_eq!(empty.of("browser_navigate"), None);
    }

    #[test]
    fn an_sse_framed_answer_reads_the_same_as_a_plain_one() {
        let plain = parse_body(r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#).unwrap();
        let framed = parse_body("event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"ok\":true}}\n\n").unwrap();
        assert_eq!(plain, framed);
    }

    #[test]
    fn a_stray_newline_in_the_providers_prose_does_not_lose_the_handshake() {
        // The real Unzoo service's instructions field carries raw newlines.
        let value = parse_body("{\"result\":{\"instructions\":\"line one\nline two\"}}").unwrap();
        assert!(value["result"]["instructions"].as_str().unwrap().contains("line one"));
    }
}
