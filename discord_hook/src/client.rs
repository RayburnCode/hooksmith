use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};

use hooksmith_core::{HttpClient, WebhookSender};

use crate::{WebhookError, WebhookMessage};

/// Validate that a webhook URL is HTTPS and targets the Discord API.
///
/// This runs at construction time so misconfigured or accidentally-HTTP URLs
/// are caught before the first request is ever made.
fn validate_url(url: &str) -> Result<(), WebhookError> {
    if !url.starts_with("https://") {
        return Err(WebhookError::InvalidUrl {
            reason: "webhook URL must use HTTPS",
        });
    }
    if !url.contains("discord.com/api/webhooks/") {
        return Err(WebhookError::InvalidUrl {
            reason: "URL must target discord.com/api/webhooks/",
        });
    }
    Ok(())
}

/// An async Discord webhook client.
///
/// # Example
///
/// ```rust,no_run
/// use discord_hook::{WebhookClient, WebhookMessage, Embed};
///
/// #[tokio::main]
/// async fn main() -> Result<(), discord_hook::WebhookError> {
///     let client = WebhookClient::new("https://discord.com/api/webhooks/ID/TOKEN")?;
///
///     let embed = Embed::builder()
///         .title("Hello from discord_hook!")
///         .description("A rich embed sent via webhook.")
///         .color(0x5865F2)
///         .field("Version", "0.1.0", true)
///         .build();
///
///     let message = WebhookMessage::builder()
///         .username("MyBot")
///         .embed(embed)
///         .build()?;
///
///     client.send(&message).await
/// }
/// ```
pub struct WebhookClient {
    /// The full webhook URL stored as a secret so it is never accidentally
    /// logged or included in debug output.  Call `.expose_secret()` only at
    /// the point where the URL is needed for an HTTP request.
    url: SecretString,
    client: HttpClient,
}

impl WebhookClient {
    /// Create a new client with a default [`reqwest::Client`].
    ///
    /// # Errors
    ///
    /// Returns [`WebhookError::InvalidUrl`] if the URL does not use HTTPS or
    /// does not target `discord.com/api/webhooks/`.
    pub fn new(url: impl Into<String>) -> Result<Self, WebhookError> {
        let url = url.into();
        validate_url(&url)?;
        Ok(Self { url: SecretString::from(url), client: HttpClient::new() })
    }

    /// Create a client that reuses a pre-configured [`reqwest::Client`].
    ///
    /// Useful when you want to share a client (connection pool, middleware, etc.)
    /// across your application.
    ///
    /// # Errors
    ///
    /// Returns [`WebhookError::InvalidUrl`] if the URL does not use HTTPS or
    /// does not target `discord.com/api/webhooks/`.
    pub fn with_client(url: impl Into<String>, client: Client) -> Result<Self, WebhookError> {
        let url = url.into();
        validate_url(&url)?;
        Ok(Self { url: SecretString::from(url), client: HttpClient::with_reqwest(client) })
    }

    /// Send a [`WebhookMessage`] to Discord.
    ///
    /// Uses `?wait=true` so Discord confirms the message was saved before
    /// responding — errors that would otherwise be silently dropped (e.g. bad
    /// embed structure) are surfaced as [`WebhookError::ApiError`].
    ///
    /// # Errors
    ///
    /// - [`WebhookError::Http`] – transport-level failure.
    /// - [`WebhookError::RateLimited`] – Discord returned HTTP 429; the
    ///   `retry_after_ms` field tells you how long to wait.
    /// - [`WebhookError::ApiError`] – any other non-2xx response.
    pub async fn send(&self, message: &WebhookMessage) -> Result<(), WebhookError> {
        self.execute(message, None).await
    }

    /// Send a [`WebhookMessage`] into a specific thread.
    ///
    /// Pass the thread's snowflake ID as `thread_id`.  The thread will be
    /// automatically unarchived if needed.
    pub async fn send_to_thread(
        &self,
        message: &WebhookMessage,
        thread_id: &str,
    ) -> Result<(), WebhookError> {
        self.execute(message, Some(thread_id)).await
    }

    async fn execute(
        &self,
        message: &WebhookMessage,
        thread_id: Option<&str>,
    ) -> Result<(), WebhookError> {
        // Always use wait=true so Discord confirms the message was saved.
        // Errors that would otherwise be silently dropped (e.g. bad embed
        // structure) are surfaced as ApiError instead.
        let mut url = format!("{}?wait=true", self.url.expose_secret());
        if let Some(tid) = thread_id {
            url.push_str("&thread_id=");
            url.push_str(tid);
        }

        let response = self.client.post_json(&url, message).await?;

        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after_ms = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<f64>().ok())
                .map(|secs| (secs * 1000.0) as u64)
                .unwrap_or(1_000);

            return Err(WebhookError::RateLimited { retry_after_ms });
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| status.to_string());
            return Err(WebhookError::ApiError { status: status.as_u16(), message: body });
        }

        Ok(())
    }
}

impl std::fmt::Debug for WebhookClient {
    /// Formats `WebhookClient` for debug output **without exposing the token**.
    ///
    /// The webhook URL is `https://discord.com/api/webhooks/{id}/{token}`.
    /// This implementation shows everything up to and including the webhook ID
    /// but replaces the token segment with `<REDACTED>`, so you can identify
    /// which webhook is configured without leaking the secret.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let raw = self.url.expose_secret();
        let redacted = match raw.rfind('/') {
            Some(idx) => format!("{}/", &raw[..idx]),
            None => String::new(),
        };
        f.debug_struct("WebhookClient")
            .field("url", &format_args!("{}<REDACTED>", redacted))
            .finish_non_exhaustive()
    }
}

impl WebhookSender for WebhookClient {
    type Message = WebhookMessage;
    type Error = WebhookError;

    /// Send a [`WebhookMessage`], satisfying the generic [`WebhookSender`] trait.
    ///
    /// Identical to the inherent [`WebhookClient::send`] method — use whichever
    /// is more convenient.  The trait is useful when writing code that is
    /// generic over notification backends.
    fn send(
        &self,
        message: &WebhookMessage,
    ) -> impl std::future::Future<Output = Result<(), WebhookError>> + Send {
        self.execute(message, None)
    }
}
