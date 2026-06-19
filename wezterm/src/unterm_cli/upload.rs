//! `unterm-cli upload <file>` — upload a local file to a user-configured
//! object storage bucket (Aliyun OSS / Tencent COS / Qiniu Kodo) via the
//! running Unterm GUI's MCP server.
//!
//! Config schema lives in `wezterm-gui/src/mcp/upload.rs`. The default
//! provider and credentials are read from `~/.unterm/upload.json`.

use super::client::McpClient;
use super::output::print_json;
use anyhow::{anyhow, Result};
use clap::{Args, Subcommand, ValueHint};
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Debug, Args, Clone)]
pub struct UploadCommand {
    #[command(subcommand)]
    pub action: Option<UploadAction>,

    /// Local file to upload (default action when no subcommand is given).
    /// Required unless invoking `upload config-path` or `upload setup`.
    #[arg(value_hint = ValueHint::FilePath)]
    pub file: Option<PathBuf>,

    /// Override the configured default provider. Allowed: `oss`, `cos`, `qiniu`.
    #[arg(short = 'p', long = "provider")]
    pub provider: Option<String>,

    /// Override the destination object key. If omitted, the server
    /// derives one from the basename + timestamp + per-provider path_prefix.
    #[arg(short = 'k', long = "key")]
    pub key: Option<String>,

    /// Print only the resulting URL on stdout (suitable for piping into
    /// `pbcopy` / `xclip` / `clip.exe`). Suppresses any other output.
    #[arg(long = "raw")]
    pub raw: bool,
}

#[derive(Debug, Subcommand, Clone)]
pub enum UploadAction {
    /// Print the absolute path of the upload config file
    /// (`~/.unterm/upload.json`) and exit. Useful for `unterm-cli upload
    /// config-path | xargs $EDITOR` to open it for editing.
    #[command(name = "config-path")]
    ConfigPath,
    /// Create a starter upload config at `~/.unterm/upload.json`.
    Setup {
        /// Overwrite an existing upload config.
        #[arg(long)]
        force: bool,
    },
}

pub fn run(cmd: UploadCommand, json_out: bool) -> Result<()> {
    match cmd.action {
        Some(UploadAction::ConfigPath) => {
            let path = config_path();
            println!("{}", path.display());
            return Ok(());
        }
        Some(UploadAction::Setup { force }) => {
            let path = write_config_template(force)?;
            if json_out {
                print_json(&json!({ "path": path, "created": true }));
            } else {
                println!("{}", path.display());
            }
            return Ok(());
        }
        None => {}
    }

    let file = cmd
        .file
        .ok_or_else(|| anyhow!("missing FILE argument (use `unterm-cli upload <path>`)"))?;

    let abs = file
        .canonicalize()
        .map_err(|e| anyhow!("resolve {:?}: {}", file, e))?;

    let mut params = json!({ "path": abs.to_string_lossy() });
    if let Some(p) = cmd.provider.as_deref() {
        params["provider"] = Value::String(p.to_string());
    }
    if let Some(k) = cmd.key.as_deref() {
        params["key"] = Value::String(k.to_string());
    }

    let mut client = McpClient::connect()?;
    let result = client.call("upload.file", params)?;

    if cmd.raw {
        let url = result
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("upload.file did not return a url: {}", result))?;
        println!("{}", url);
    } else if json_out {
        print_json(&result);
    } else {
        let url = result
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("<no url>");
        let provider = result
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let key = result.get("key").and_then(|v| v.as_str()).unwrap_or("?");
        let size = result.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
        println!("URL:      {}", url);
        println!("Provider: {}", provider);
        println!("Key:      {}", key);
        println!("Size:     {} bytes", size);
    }
    Ok(())
}

fn config_path() -> PathBuf {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("upload.json")
}

fn write_config_template(force: bool) -> Result<PathBuf> {
    let path = config_path();
    if path.exists() && !force {
        return Err(anyhow!(
            "{} already exists; pass `unterm-cli upload setup --force` to overwrite",
            path.display()
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, upload_config_template())?;
    Ok(path)
}

fn upload_config_template() -> &'static str {
    r#"{
  "default_provider": "oss",
  "oss": {
    "endpoint": "oss-cn-hangzhou.aliyuncs.com",
    "bucket": "your-bucket",
    "access_key_id": "your-access-key-id",
    "access_key_secret": "your-access-key-secret",
    "url_prefix": "https://cdn.example.com",
    "path_prefix": "unterm/"
  },
  "cos": {
    "secret_id": "your-secret-id",
    "secret_key": "your-secret-key",
    "bucket": "your-bucket-1250000000",
    "region": "ap-shanghai",
    "url_prefix": "https://cdn.example.com",
    "path_prefix": "unterm/"
  },
  "qiniu": {
    "access_key": "your-access-key",
    "secret_key": "your-secret-key",
    "bucket": "your-bucket",
    "domain": "https://cdn.example.com",
    "path_prefix": "unterm/"
  }
}
"#
}

#[cfg(test)]
mod tests {
    use super::upload_config_template;

    #[test]
    fn upload_setup_template_is_valid_json() {
        let value: serde_json::Value = serde_json::from_str(upload_config_template()).unwrap();
        assert_eq!(value["default_provider"], "oss");
        assert!(value.get("oss").is_some());
        assert!(value.get("cos").is_some());
        assert!(value.get("qiniu").is_some());
    }
}
