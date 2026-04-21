use thiserror::Error;

/// Transport-level errors shared across all hooksmith service crates.
///
/// Service-specific error types should wrap this via `#[from]` so that generic
/// code can pattern-match on network or JSON failures without knowing which
/// service produced them.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Debug, thiserror::Error)]
/// pub enum DiscordError {
///     #[error(transparent)]
///     Core(#[from] hooksmith_core::CoreError),
///
///     #[error("Discord API error {status}: {body}")]
///     Api { status: u16, body: String },
/// }
/// ```
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum CoreError {
    /// An HTTP or network-level failure from the underlying [`reqwest`] client.
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    /// JSON serialization or deserialization failed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
