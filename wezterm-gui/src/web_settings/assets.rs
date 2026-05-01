//! Static assets bundled into the unterm binary via `include_str!`.
//!
//! The SPA itself is plain HTML + Tailwind (Play CDN, vendored offline) +
//! Alpine.js (vendored offline). No build step, no JS framework with a
//! compile pass — editing `wezterm-gui/assets/settings/*` and rebuilding
//! the binary is the entire workflow.

pub const INDEX_HTML: &str = include_str!("../../assets/settings/index.html");
pub const APP_JS: &str = include_str!("../../assets/settings/app.js");
pub const STYLE_CSS: &str = include_str!("../../assets/settings/style.css");
pub const TAILWIND_JS: &str = include_str!("../../assets/settings/tailwind.js");
pub const ALPINE_JS: &str = include_str!("../../assets/settings/alpine.js");

/// Match a `/static/<name>` request to a `(content_type, body)` pair.
pub fn lookup_static(name: &str) -> Option<(&'static str, &'static str)> {
    match name {
        "app.js" => Some(("application/javascript; charset=utf-8", APP_JS)),
        "style.css" => Some(("text/css; charset=utf-8", STYLE_CSS)),
        "tailwind.js" => Some(("application/javascript; charset=utf-8", TAILWIND_JS)),
        "alpine.js" => Some(("application/javascript; charset=utf-8", ALPINE_JS)),
        _ => None,
    }
}
