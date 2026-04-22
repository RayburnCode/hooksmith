use thiserror::Error;

#[derive(Error, Debug)]
pub enum WebhookError {
    /// An underlying HTTP transport error from reqwest.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// The message had neither content nor any embeds.
    #[error("Message must have at least content or one embed")]
    EmptyMessage,

    /// Discord rate-limited the request. The caller should wait `retry_after_ms`
    /// milliseconds before retrying.
    #[error("Rate limited by Discord: retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    /// Discord returned a non-success status code.
    #[error("Discord API error (HTTP {status}): {message}")]
    ApiError { status: u16, message: String },

    /// A value could not be serialized to JSON.
    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    /// The provided webhook URL is invalid or insecure.
    ///
    /// URLs must use HTTPS and target `discord.com/api/webhooks/`.
    #[error("Invalid webhook URL: {reason}")]
    InvalidUrl { reason: &'static str },

    /// The thread ID is not a valid Discord snowflake.
    ///
    /// Thread IDs must be non-empty strings of ASCII digits.  Non-numeric
    /// characters are rejected to prevent URL manipulation attacks.
    #[error("Invalid thread ID: must be a non-empty numeric Discord snowflake")]
    InvalidThreadId,
}
