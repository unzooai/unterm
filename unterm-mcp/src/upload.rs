//! Upload a local file to a user-configured object storage bucket
//! (Aliyun OSS / Tencent COS / Qiniu Kodo) and return a public URL.
//!
//! Designed to pair with `capture.*` so an AI agent can take a screenshot,
//! then call `upload.file` to get a public URL it can embed in chat /
//! issue tracker / doc without the user having to drag the PNG around.
//!
//! Credentials live in `~/.unterm/upload.json` (see [`UploadConfig`]).
//! No credentials are ever logged or returned in the MCP response —
//! only the resulting URL, the provider name, and the storage key.
//!
//! Signing is hand-rolled (HMAC-SHA1 for all three providers) so we
//! don't depend on any of the vendor SDKs, which are all multi-MB.

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type HmacSha1 = Hmac<Sha1>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UploadConfig {
    /// Provider used when the caller doesn't pass `provider` explicitly.
    #[serde(default)]
    pub default_provider: Option<String>,
    #[serde(default)]
    pub oss: Option<OssConfig>,
    #[serde(default)]
    pub cos: Option<CosConfig>,
    #[serde(default)]
    pub qiniu: Option<QiniuConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OssConfig {
    /// e.g. `oss-cn-hangzhou.aliyuncs.com` (no scheme, no leading dot).
    pub endpoint: String,
    pub bucket: String,
    pub access_key_id: String,
    pub access_key_secret: String,
    /// Optional public URL prefix (e.g. a CDN domain). If omitted, the
    /// returned URL falls back to `https://<bucket>.<endpoint>/<key>`.
    #[serde(default)]
    pub url_prefix: Option<String>,
    /// Optional key prefix prepended to every upload (e.g. `unterm/`).
    #[serde(default)]
    pub path_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosConfig {
    pub secret_id: String,
    pub secret_key: String,
    pub bucket: String,
    /// e.g. `ap-shanghai`.
    pub region: String,
    #[serde(default)]
    pub url_prefix: Option<String>,
    #[serde(default)]
    pub path_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QiniuConfig {
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
    /// Public CDN / S3 domain — required, because Qiniu's upload
    /// endpoint isn't itself the read endpoint.
    pub domain: String,
    #[serde(default)]
    pub path_prefix: Option<String>,
}

/// Location of the persisted config. The user creates this by hand
/// or with `unterm-cli upload setup`.
pub fn config_path() -> PathBuf {
    unterm_protocol::state_path("upload.json")
        .unwrap_or_else(|| PathBuf::from(".unterm").join("upload.json"))
}

pub fn load_config() -> Result<UploadConfig> {
    let path = config_path();
    let bytes = fs::read(&path).with_context(|| {
        format!(
            "read upload config at {} (create it with provider credentials before uploading)",
            path.display()
        )
    })?;
    let cfg: UploadConfig = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse upload config at {}", path.display()))?;
    Ok(cfg)
}

/// MCP `upload.file` handler.
///
/// Params:
/// - `path` (string, required): local file to upload.
/// - `provider` (string, optional): `oss` | `cos` | `qiniu`. Defaults to
///   `default_provider` from the config.
/// - `key` (string, optional): destination object key. If omitted, derived
///   from the file basename plus a timestamp and the provider's `path_prefix`.
///
/// Returns: `{ "url": "...", "provider": "...", "key": "...", "size": N }`.
pub fn upload(params: &Value) -> Result<Value> {
    let path = params
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("`path` parameter required"))?;

    let cfg = load_config()?;

    let provider_name = params
        .get("provider")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| cfg.default_provider.clone())
        .ok_or_else(|| {
            anyhow!(
                "no `provider` in call and no `default_provider` in {}",
                config_path().display()
            )
        })?;

    let key = params
        .get("key")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| derive_key(path, &provider_name, &cfg));

    let bytes = fs::read(path).with_context(|| format!("read local file {}", path))?;
    let content_type = mime_for(path);

    let url = match provider_name.as_str() {
        "oss" => upload_oss(
            cfg.oss
                .as_ref()
                .ok_or_else(|| anyhow!("`oss` block missing in upload config"))?,
            &key,
            &bytes,
            &content_type,
        )?,
        "cos" => upload_cos(
            cfg.cos
                .as_ref()
                .ok_or_else(|| anyhow!("`cos` block missing in upload config"))?,
            &key,
            &bytes,
            &content_type,
        )?,
        "qiniu" => upload_qiniu(
            cfg.qiniu
                .as_ref()
                .ok_or_else(|| anyhow!("`qiniu` block missing in upload config"))?,
            &key,
            &bytes,
        )?,
        other => bail!("unknown provider {:?}; expected oss | cos | qiniu", other),
    };

    Ok(json!({
        "url": url,
        "provider": provider_name,
        "key": key,
        "size": bytes.len(),
    }))
}

fn derive_key(local_path: &str, provider: &str, cfg: &UploadConfig) -> String {
    let basename = Path::new(local_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "upload.bin".to_string());
    let ts = chrono::Local::now().format("%Y/%m/%d/%H%M%S_%3f");
    let prefix = match provider {
        "oss" => cfg.oss.as_ref().and_then(|c| c.path_prefix.clone()),
        "cos" => cfg.cos.as_ref().and_then(|c| c.path_prefix.clone()),
        "qiniu" => cfg.qiniu.as_ref().and_then(|c| c.path_prefix.clone()),
        _ => None,
    }
    .unwrap_or_default();
    let prefix = prefix.trim_matches('/');
    if prefix.is_empty() {
        format!("{}_{}", ts, basename)
    } else {
        format!("{}/{}_{}", prefix, ts, basename)
    }
}

fn mime_for(path: &str) -> String {
    let ext = Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png".into(),
        "jpg" | "jpeg" => "image/jpeg".into(),
        "gif" => "image/gif".into(),
        "webp" => "image/webp".into(),
        "svg" => "image/svg+xml".into(),
        "bmp" => "image/bmp".into(),
        "ico" => "image/x-icon".into(),
        "mp4" => "video/mp4".into(),
        "webm" => "video/webm".into(),
        "mov" => "video/quicktime".into(),
        "pdf" => "application/pdf".into(),
        "txt" | "log" | "md" => "text/plain; charset=utf-8".into(),
        "json" => "application/json".into(),
        "html" | "htm" => "text/html; charset=utf-8".into(),
        _ => "application/octet-stream".into(),
    }
}

// ============================================================================
// Aliyun OSS
// https://help.aliyun.com/zh/oss/developer-reference/include-signatures-in-the-authorization-header
//
// PUT https://<bucket>.<endpoint>/<key>
//   Date:          <RFC1123 GMT>
//   Content-Type:  <mime>
//   Authorization: OSS <AccessKeyId>:<base64(HMAC-SHA1(secret, StringToSign))>
//
// StringToSign:
//   PUT\n\n<Content-Type>\n<Date>\n/<bucket>/<key>
// ============================================================================
fn upload_oss(cfg: &OssConfig, key: &str, body: &[u8], content_type: &str) -> Result<String> {
    let date = http_date(SystemTime::now());
    let canonical = format!("/{}/{}", cfg.bucket, key);
    let to_sign = format!("PUT\n\n{}\n{}\n{}", content_type, date, canonical);
    let signature = hmac_sha1_b64(cfg.access_key_secret.as_bytes(), to_sign.as_bytes());
    let auth = format!("OSS {}:{}", cfg.access_key_id, signature);

    let endpoint = cfg
        .endpoint
        .trim()
        .trim_end_matches('/')
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let url = format!("https://{}.{}/{}", cfg.bucket, endpoint, key);

    let client = http_client()?;
    let resp = client
        .put(&url)
        .header("Date", date)
        .header("Content-Type", content_type)
        .header("Authorization", auth)
        .body(body.to_vec())
        .send()
        .context("OSS PUT failed")?;
    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().unwrap_or_default();
        bail!("OSS upload returned HTTP {}: {}", status, body_text);
    }

    Ok(match &cfg.url_prefix {
        Some(p) => format!("{}/{}", p.trim_end_matches('/'), key),
        None => url,
    })
}

// ============================================================================
// Tencent COS
// https://cloud.tencent.com/document/product/436/7778
//
// PUT https://<bucket>.cos.<region>.myqcloud.com/<key>
//   Authorization: q-sign-algorithm=sha1&q-ak=...&q-sign-time=...
//                  &q-key-time=...&q-header-list=&q-url-param-list=&q-signature=...
//
// Step 1: SignKey      = HMAC-SHA1(SecretKey, KeyTime)
// Step 2: HttpString   = "<method>\n<path>\n<params>\n<headers>\n"
// Step 3: StringToSign = "sha1\n<KeyTime>\n<sha1_hex(HttpString)>\n"
// Step 4: Signature    = HMAC-SHA1(SignKey, StringToSign)
//
// We pass no extra signed headers / params, so header-list and url-param-list
// are both empty.
// ============================================================================
fn upload_cos(cfg: &CosConfig, key: &str, body: &[u8], content_type: &str) -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let key_time = format!("{};{}", now, now + 3600);
    let sign_key = hmac_sha1_hex(cfg.secret_key.as_bytes(), key_time.as_bytes());

    let path = format!("/{}", key);
    let http_string = format!("put\n{}\n\n\n", path);
    let http_string_sha1 = sha1_hex(http_string.as_bytes());
    let string_to_sign = format!("sha1\n{}\n{}\n", key_time, http_string_sha1);
    let signature = hmac_sha1_hex(sign_key.as_bytes(), string_to_sign.as_bytes());

    let authorization = format!(
        "q-sign-algorithm=sha1&q-ak={}&q-sign-time={}&q-key-time={}\
         &q-header-list=&q-url-param-list=&q-signature={}",
        cfg.secret_id, key_time, key_time, signature
    );

    let url = format!(
        "https://{}.cos.{}.myqcloud.com/{}",
        cfg.bucket, cfg.region, key
    );

    let client = http_client()?;
    let resp = client
        .put(&url)
        .header("Authorization", authorization)
        .header("Content-Type", content_type)
        .body(body.to_vec())
        .send()
        .context("COS PUT failed")?;
    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().unwrap_or_default();
        bail!("COS upload returned HTTP {}: {}", status, body_text);
    }

    Ok(match &cfg.url_prefix {
        Some(p) => format!("{}/{}", p.trim_end_matches('/'), key),
        None => url,
    })
}

// ============================================================================
// Qiniu Kodo
// https://developer.qiniu.com/kodo/1208/upload-token
//
// 1. PutPolicy   = { "scope": "<bucket>:<key>", "deadline": <unix_ts> }
// 2. Encoded     = base64UrlSafe(JSON(PutPolicy))
// 3. Sign        = base64UrlSafe(HMAC-SHA1(SecretKey, Encoded))
// 4. UploadToken = "<AccessKey>:<Sign>:<Encoded>"
//
// Then multipart/form-data POST to https://upload.qiniup.com/ with fields:
//   key   = <key>
//   token = <UploadToken>
//   file  = <bytes>
//
// Public URL = "<domain>/<key>" (caller-supplied domain, since Qiniu
// upload endpoint isn't itself the read endpoint).
// ============================================================================
fn upload_qiniu(cfg: &QiniuConfig, key: &str, body: &[u8]) -> Result<String> {
    let deadline = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + 3600;
    let put_policy = json!({
        "scope": format!("{}:{}", cfg.bucket, key),
        "deadline": deadline,
    });
    let policy_json = serde_json::to_string(&put_policy)?;
    let encoded_policy =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(policy_json.as_bytes());

    let mut mac =
        HmacSha1::new_from_slice(cfg.secret_key.as_bytes()).expect("HMAC accepts any key length");
    mac.update(encoded_policy.as_bytes());
    let signature =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

    let token = format!("{}:{}:{}", cfg.access_key, signature, encoded_policy);

    let form = reqwest::blocking::multipart::Form::new()
        .text("key", key.to_string())
        .text("token", token)
        .part(
            "file",
            reqwest::blocking::multipart::Part::bytes(body.to_vec()).file_name(key.to_string()),
        );

    let client = http_client()?;
    let resp = client
        .post("https://upload.qiniup.com/")
        .multipart(form)
        .send()
        .context("Qiniu upload POST failed")?;
    let status = resp.status();
    let resp_text = resp.text().unwrap_or_default();
    if !status.is_success() {
        bail!("Qiniu upload returned HTTP {}: {}", status, resp_text);
    }

    Ok(format!("{}/{}", cfg.domain.trim_end_matches('/'), key))
}

// ----------------------------------------------------------------------------
// helpers
// ----------------------------------------------------------------------------

fn http_client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .context("build reqwest blocking client")
}

fn hmac_sha1_b64(key: &[u8], msg: &[u8]) -> String {
    let mut mac = HmacSha1::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(msg);
    base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
}

fn hmac_sha1_hex(key: &[u8], msg: &[u8]) -> String {
    let mut mac = HmacSha1::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(msg);
    hex_encode(&mac.finalize().into_bytes())
}

fn sha1_hex(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    hex_encode(&hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// RFC 1123 / RFC 7231 IMF-fixdate. Aliyun OSS rejects anything else.
/// Always English, regardless of process locale.
fn http_date(t: SystemTime) -> String {
    let dt: chrono::DateTime<chrono::Utc> = t.into();
    // %a, %d %b %Y %H:%M:%S GMT — chrono's %a/%b emit English by default,
    // so this stays stable even with unstable-locales enabled elsewhere.
    dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_date_matches_rfc1123() {
        // Just shape check — the actual time floats.
        let s = http_date(SystemTime::now());
        assert!(s.ends_with(" GMT"));
        assert_eq!(s.len(), 29, "expected 29-char IMF-fixdate, got {:?}", s);
    }

    #[test]
    fn hmac_sha1_known_vector() {
        // RFC 2202 test case 1.
        let mac = hmac_sha1_b64(&[0x0b; 20], b"Hi There");
        // expected = base64(HMAC-SHA1) of the known hex
        // hex: b617318655057264e28bc0b6fb378c8ef146be00
        let expected = base64::engine::general_purpose::STANDARD
            .encode(hex_decode("b617318655057264e28bc0b6fb378c8ef146be00"));
        assert_eq!(mac, expected);
    }

    fn hex_decode(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn sha1_known_vector() {
        // "abc" → a9993e364706816aba3e25717850c26c9cd0d89d
        assert_eq!(sha1_hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn derive_key_uses_basename_when_no_prefix() {
        let cfg = UploadConfig::default();
        let k = derive_key("/tmp/foo.png", "oss", &cfg);
        assert!(k.ends_with("_foo.png"), "got {}", k);
        assert!(!k.starts_with('/'));
    }

    #[test]
    fn derive_key_honors_path_prefix() {
        let cfg = UploadConfig {
            oss: Some(OssConfig {
                endpoint: "x".into(),
                bucket: "b".into(),
                access_key_id: "".into(),
                access_key_secret: "".into(),
                url_prefix: None,
                path_prefix: Some("unterm/".into()),
            }),
            ..Default::default()
        };
        let k = derive_key("/tmp/foo.png", "oss", &cfg);
        assert!(k.starts_with("unterm/"), "got {}", k);
        assert!(k.ends_with("_foo.png"), "got {}", k);
    }
}
