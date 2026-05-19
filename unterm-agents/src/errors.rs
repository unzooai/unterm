use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("envelope failed to parse: {0}")]
    ParseFailed(String),

    #[error("unsupported signature algorithm: {0}")]
    UnsupportedSigAlg(String),

    #[error("signature key id {0:?} not in trusted set; envelope rejected")]
    UnknownKeyId(String),

    #[error("envelope signature verification failed: {0}")]
    BadSignature(String),

    #[error("envelope expired at {0}; refusing to use it")]
    Expired(String),

    #[error("this envelope requires Unterm >= {need}; you have {have}")]
    ClientTooOld { have: String, need: String },

    #[error("trusted-keys config corrupt or empty: {0}")]
    TrustedKeysCorrupt(String),

    #[error("network fetch failed: {0}")]
    Fetch(String),

    #[error("on-disk cache I/O failed: {0}")]
    Cache(String),

    #[error("no manifests available — cache empty, network down, and baked fallback corrupt")]
    NoSource,

    #[error("agent {0:?} not found in current manifest set")]
    UnknownAgent(String),

    #[error("setting {0:?} not in this agent's schema")]
    UnknownSetting(String),

    #[error("setting {key:?}: invalid value: {reason}")]
    InvalidSettingValue { key: String, reason: String },

    #[error("install step failed (exit {exit:?}): {detail}")]
    InstallFailed { exit: Option<i32>, detail: String },

    #[error("auth step failed: {0}")]
    AuthFailed(String),

    #[error("storage format {0:?} not supported by this build")]
    UnsupportedFormat(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde_json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, AgentError>;
