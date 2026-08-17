//! Run the settings page on its own, against an isolated state directory.
//!
//! ```text
//! cargo run -p unterm-settings --example serve
//! ```
//!
//! There is a real Unterm on this machine and it serves the same page from a
//! binary that was built weeks ago; this runs the one in the working tree
//! instead. It exists so a change to the page can be opened and looked at
//! without building, signing and launching the whole application — and so a
//! browser driving it in a test is driving the current code.
//!
//! The state directory is a temporary one, so nothing here touches the
//! user's own instances, tasks or provider bindings.

fn main() {
    // An explicit state directory is honoured, so this can be pointed at a
    // running Core's — which is how the approval flow is exercised end to
    // end: the agent asks through the Core, the person answers here.
    let dir = match std::env::var_os("UNTERM_STATE_DIR") {
        Some(dir) => std::path::PathBuf::from(dir),
        None => {
            let dir =
                std::env::temp_dir().join(format!("unterm-settings-preview-{}", std::process::id()));
            std::env::set_var("UNTERM_STATE_DIR", &dir);
            dir
        }
    };
    std::fs::create_dir_all(&dir).expect("make a state directory");

    // The page bootstraps its token from the instance record, so there has to
    // be one. Port 0 for the MCP port: this preview serves the settings page
    // and nothing is expected to connect to an MCP server that isn't there.
    let info = unterm_services::server_info::write_initial(0).expect("write an instance record");
    let port = unterm_settings::server::start_web_settings_server(info.auth_token.clone());
    let _ = unterm_services::server_info::set_http_port(port);

    println!("state dir:  {}", dir.display());
    println!("token:      {}", info.auth_token);
    println!("settings:   http://127.0.0.1:{port}/");
    println!("Ctrl-C to stop.");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
