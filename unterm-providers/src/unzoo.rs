//! The Unzoo browser, as a provider.
//!
//! Unzoo runs a service that speaks MCP over HTTP and writes the port it
//! chose into its own config directory. Reading that file is the whole of
//! discovery: a user who installed the browser gets a working binding without
//! configuring anything, and a browser that restarted on a different port is
//! found rather than missed.
//!
//! What this deliberately does *not* do is talk to the browser any other way.
//! Unzoo also exposes a CDP shim, and driving that would bypass every
//! capability boundary here — the profile isolation, the leases, the audit
//! trail — while looking like it worked. Browser work goes through this
//! provider or it does not happen.
//!
//! **Identity.** Loopback proves a process on this machine answered, not that
//! it is the one the user bound. The service reports its name and version in
//! the MCP handshake; that is pinned on first bind and compared afterwards.
//! It is trust-on-first-use, which is worth stating plainly: it detects a
//! provider that changed, not one that was wrong from the start.

use crate::mcp_http::{Families, HttpMcpProvider};
use crate::{Capability, Endpoint, Failure, ProviderManifest};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The provider id Unzoo is known by.
pub const ID: &str = "unzoo";

/// Where Unzoo keeps the port it chose, per platform.
///
/// The file, not a constant: this is what makes a restarted browser findable
/// and a stale port harmless.
fn port_file() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    let home = PathBuf::from(home);
    let candidates = if cfg!(target_os = "macos") {
        vec![home.join("Library/Application Support/Unzoo Browser/config/rest-port")]
    } else if cfg!(target_os = "windows") {
        let roaming = std::env::var("APPDATA").map(PathBuf::from).ok();
        let mut paths = Vec::new();
        if let Some(roaming) = roaming {
            paths.push(roaming.join("Unzoo Browser/config/rest-port"));
        }
        paths.push(home.join("AppData/Roaming/Unzoo Browser/config/rest-port"));
        paths
    } else {
        vec![
            home.join(".config/unzoo-browser/rest-port"),
            home.join(".config/Unzoo Browser/config/rest-port"),
        ]
    };
    candidates.into_iter().find(|path| path.exists())
}

/// The endpoint Unzoo advertised, if it is running.
pub fn advertised_endpoint() -> Option<Endpoint> {
    let path = port_file()?;
    let text = std::fs::read_to_string(path).ok()?;
    let port: u16 = text.trim().parse().ok()?;
    if port == 0 {
        return None;
    }
    Some(Endpoint::Http {
        // Loopback because the file is Unzoo's statement about its own local
        // service; a port file is not authority to reach across a network.
        url: format!("http://127.0.0.1:{port}/mcp"),
    })
}

/// Unzoo as a manifest, when it can be found.
pub fn advertised() -> Vec<ProviderManifest> {
    let Some(endpoint) = advertised_endpoint() else {
        return Vec::new();
    };
    vec![ProviderManifest {
        id: ID.to_string(),
        name: "Unzoo Browser".to_string(),
        endpoint,
        protocols: crate::discovery::PROTOCOLS
            .iter()
            .map(|version| version.to_string())
            .collect(),
        // A starting point, replaced by what the handshake actually reports.
        capabilities: Capability::ALL.to_vec(),
        families: families(),
        source: "unzoo:rest-port".to_string(),
        pinned: crate::discovery::pinned(ID),
    }]
}

/// Which family an Unzoo tool belongs to.
///
/// The mapping is by prefix because that is how the tool surface is actually
/// organised, and an unrecognised prefix maps to nothing rather than to a
/// guess: a new family appearing in an Unzoo update should be unusable until
/// somebody decides which permission it belongs under, not silently covered
/// by whichever lease is closest.
pub const FAMILIES: &[(&str, Capability)] = &[
    // Driving pages.
    ("browser", Capability::Browser),
    ("tab", Capability::Browser),
    ("page", Capability::Browser),
    ("extract", Capability::Browser),
    ("media", Capability::Browser),
    ("youtube", Capability::Browser),
    ("ytdlp", Capability::Browser),
    // Whose browser this is.
    ("profile", Capability::Profile),
    ("cookie", Capability::Profile),
    ("session", Capability::Profile),
    // The machine around it.
    ("human", Capability::Computer),
    ("file", Capability::Computer),
    ("download", Capability::Computer),
    ("workspace", Capability::Computer),
    ("upload", Capability::Computer),
    // service_*, mcp_*, tool_* describe the provider itself; they are not a
    // capability anyone leases, so they are absent rather than mapped.
];

/// The table, as discovery hands it to a client.
pub fn families() -> BTreeMap<String, Capability> {
    FAMILIES
        .iter()
        .map(|(prefix, capability)| (prefix.to_string(), *capability))
        .collect()
}

/// Which family an Unzoo tool belongs to.
pub fn family_of(tool: &str) -> Option<Capability> {
    families()
        .get(tool.split('_').next().unwrap_or_default())
        .copied()
}

/// A client for the running Unzoo service.
pub fn provider(manifest: &ProviderManifest) -> Result<HttpMcpProvider, Failure> {
    match &manifest.endpoint {
        Endpoint::Http { url } => Ok(HttpMcpProvider::new(
            ID,
            url,
            Families::Prefixes(families()),
        )),
        other => Err(Failure::Incompatible(format!(
            "Unzoo speaks HTTP; this manifest says {other:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_are_sorted_into_the_families_a_person_would_expect() {
        // The three questions a user is actually answering: may it drive my
        // browser, may it use my logins, may it touch my machine.
        for (tool, expected) in [
            ("browser_navigate", Capability::Browser),
            ("tab_create", Capability::Browser),
            ("page_analyze", Capability::Browser),
            ("extract_table", Capability::Browser),
            ("media_download", Capability::Browser),
            ("profile_list", Capability::Profile),
            ("cookie_get_all", Capability::Profile),
            ("session_export", Capability::Profile),
            ("human_click", Capability::Computer),
            ("file_upload", Capability::Computer),
            ("download_url", Capability::Computer),
        ] {
            assert_eq!(family_of(tool), Some(expected), "{tool}");
        }
    }

    #[test]
    fn cookies_are_not_part_of_driving_the_browser() {
        // The distinction the whole capability split exists for: letting an
        // agent open a page is not letting it read who you are logged in as.
        assert_ne!(family_of("cookie_get_all"), family_of("browser_navigate"));
        assert_ne!(family_of("profile_switch"), family_of("browser_click"));
    }

    #[test]
    fn a_tool_family_nobody_has_classified_is_not_covered_by_anything() {
        // A new family in an Unzoo update must be unusable until somebody
        // decides what it is, rather than falling under whichever lease is
        // nearest.
        assert_eq!(family_of("quantum_teleport"), None);
        assert_eq!(family_of("service_status"), None, "provider metadata is not a capability");
        assert_eq!(family_of(""), None);
    }

    #[test]
    fn the_endpoint_comes_from_the_file_not_from_this_source() {
        let _guard = crate::testing::env_guard();
        // Read through the same helper the real path uses, with the file
        // saying something no build would ever hardcode.
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("Library/Application Support/Unzoo Browser/config");
        std::fs::create_dir_all(&config).unwrap();
        std::fs::write(config.join("rest-port"), "51999\n").unwrap();

        let previous = std::env::var("HOME").ok();
        std::env::set_var("HOME", dir.path());
        let endpoint = advertised_endpoint();
        match previous {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }

        if cfg!(target_os = "macos") {
            assert_eq!(
                endpoint,
                Some(Endpoint::Http {
                    url: "http://127.0.0.1:51999/mcp".into()
                })
            );
        }
    }

    #[test]
    fn a_browser_that_is_not_running_is_simply_not_found() {
        let _guard = crate::testing::env_guard();
        let dir = tempfile::tempdir().unwrap();
        let previous = std::env::var("HOME").ok();
        std::env::set_var("HOME", dir.path());
        let endpoint = advertised_endpoint();
        match previous {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }
        assert_eq!(endpoint, None, "a missing port file must not become a guess");
    }


}
