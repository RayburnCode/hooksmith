use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use std::time::Duration;

use hooksmith_core::{HttpClient, RetryPolicy, WebhookSender};

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

/// Validate that a thread ID is a valid Discord snowflake (non-empty, all ASCII digits).
///
/// Thread IDs are appended to the request URL, so rejecting non-numeric characters
/// prevents URL manipulation attacks.
fn validate_thread_id(thread_id: &str) -> Result<(), WebhookError> {
    if thread_id.is_empty() || !thread_id.chars().all(|c| c.is_ascii_digit()) {
        return Err(WebhookError::InvalidThreadId);
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
#[derive(Clone)]
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
        Ok(Self {
            url: SecretString::from(url),
            client: HttpClient::new(),
        })
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
        Ok(Self {
            url: SecretString::from(url),
            client: HttpClient::with_reqwest(client),
        })
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
        validate_thread_id(thread_id)?;
        self.execute(message, Some(thread_id)).await
    }

    /// Send a [`WebhookMessage`] with automatic retries on transport failure.
    ///
    /// Uses the supplied [`RetryPolicy`] for exponential backoff. Note that
    /// Discord 429 rate-limit responses are **not** retried automatically —
    /// they are returned immediately as [`WebhookError::RateLimited`] so your
    /// application can handle the `retry_after_ms` value explicitly.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use discord_hook::{WebhookClient, WebhookMessage};
    /// use hooksmith_core::RetryPolicy;
    /// use std::time::Duration;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), discord_hook::WebhookError> {
    /// let client = WebhookClient::new("https://discord.com/api/webhooks/ID/TOKEN")?;
    /// let message = WebhookMessage::builder().content("hello").build()?;
    ///
    /// let policy = RetryPolicy { max_attempts: 4, base_delay: Duration::from_millis(250), jitter: true };
    /// client.send_with_retry(&message, &policy).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_with_retry(
        &self,
        message: &WebhookMessage,
        policy: &RetryPolicy,
    ) -> Result<(), WebhookError> {
        let url = format!("{}?wait=true", self.url.expose_secret());
        let response = self
            .client
            .post_json_with_retry(&url, message, policy)
            .await?;
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
            let body = response
                .bytes()
                .await
                .map(|b| {
                    // Cap at 4 KiB — enough to describe any API error without
                    // allowing a misbehaving endpoint to exhaust memory.
                    let slice = &b[..b.len().min(4096)];
                    String::from_utf8_lossy(slice).into_owned()
                })
                .unwrap_or_else(|_| status.to_string());
            return Err(WebhookError::ApiError {
                status: status.as_u16(),
                message: body,
            });
        }

        Ok(())
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
            let body = response
                .bytes()
                .await
                .map(|b| {
                    let slice = &b[..b.len().min(4096)];
                    String::from_utf8_lossy(slice).into_owned()
                })
                .unwrap_or_else(|_| status.to_string());
            return Err(WebhookError::ApiError {
                status: status.as_u16(),
                message: body,
            });
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

// ---------------------------------------------------------------------------
// WebhookClientBuilder
// ---------------------------------------------------------------------------

/// Builder for [`WebhookClient`] with configurable timeout settings.
///
/// Use this instead of [`WebhookClient::new`] when you need to control connect
/// or overall request timeouts rather than accepting the 30-second default.
///
/// # Example
///
/// ```rust,no_run
/// use discord_hook::WebhookClientBuilder;
/// use std::time::Duration;
///
/// # fn main() -> Result<(), discord_hook::WebhookError> {
/// let client = WebhookClientBuilder::new("https://discord.com/api/webhooks/ID/TOKEN")
///     .connect_timeout(Duration::from_secs(5))
///     .request_timeout(Duration::from_secs(15))
///     .build()?;
/// # Ok(())
/// # }
/// ```
pub struct WebhookClientBuilder {
    url: String,
    connect_timeout: Option<Duration>,
    request_timeout: Option<Duration>,
}

impl WebhookClientBuilder {
    /// Start building a client for the given Discord webhook URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            connect_timeout: None,
            // Mirror the default applied by WebhookClient::new.
            request_timeout: Some(Duration::from_secs(30)),
        }
    }

    /// Set the maximum time allowed to establish a TCP connection.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Set the maximum time allowed for a complete request/response cycle.
    ///
    /// Defaults to 30 seconds.
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = Some(timeout);
        self
    }

    /// Validate the URL and build the [`WebhookClient`].
    ///
    /// # Errors
    ///
    /// - [`WebhookError::InvalidUrl`] — URL is not a valid Discord webhook URL.
    /// - [`WebhookError::Http`] — underlying HTTP client could not be constructed
    ///   (e.g. TLS backend unavailable).
    pub fn build(self) -> Result<WebhookClient, WebhookError> {
        validate_url(&self.url)?;
        let mut builder = Client::builder();
        if let Some(t) = self.connect_timeout {
            builder = builder.connect_timeout(t);
        }
        if let Some(t) = self.request_timeout {
            builder = builder.timeout(t);
        }
        let client = builder.build().map_err(WebhookError::Http)?;
        Ok(WebhookClient {
            url: SecretString::from(self.url),
            client: HttpClient::with_reqwest(client),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_http_url() {
        let err = validate_url("http://discord.com/api/webhooks/123/abc").unwrap_err();
        assert!(matches!(err, WebhookError::InvalidUrl { .. }));
    }

    #[test]
    fn rejects_non_discord_url() {
        let err = validate_url("https://example.com/webhook").unwrap_err();
        assert!(matches!(err, WebhookError::InvalidUrl { .. }));
    }

    #[test]
    fn accepts_valid_discord_url() {
        assert!(validate_url("https://discord.com/api/webhooks/123456789/abcdef").is_ok());
    }

    #[test]
    fn client_new_propagates_invalid_url() {
        let err = WebhookClient::new("not-a-url").unwrap_err();
        assert!(matches!(err, WebhookError::InvalidUrl { .. }));
    }

    #[test]
    fn debug_output_redacts_token() {
        let client =
            WebhookClient::new("https://discord.com/api/webhooks/123456789/SECRET_TOKEN").unwrap();
        let debug = format!("{client:?}");
        assert!(
            !debug.contains("SECRET_TOKEN"),
            "token must not appear in debug output"
        );
        assert!(debug.contains("123456789"), "webhook id should be visible");
    }

    #[test]
    fn thread_id_rejects_empty() {
        assert!(matches!(
            validate_thread_id(""),
            Err(WebhookError::InvalidThreadId)
        ));
    }

    #[test]
    fn thread_id_rejects_non_numeric() {
        for bad in &["abc", "123abc", "12&34", "12/34", "12?id=1", " 123"] {
            assert!(
                matches!(validate_thread_id(bad), Err(WebhookError::InvalidThreadId)),
                "expected InvalidThreadId for {:?}",
                bad
            );
        }
    }

    #[test]
    fn thread_id_accepts_valid_snowflake() {
        assert!(validate_thread_id("1234567890123456789").is_ok());
    }

    #[test]
    fn builder_accepts_valid_url() {
        use std::time::Duration;
        let result = WebhookClientBuilder::new("https://discord.com/api/webhooks/123456789/abcdef")
            .connect_timeout(Duration::from_secs(5))
            .request_timeout(Duration::from_secs(15))
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn builder_rejects_invalid_url() {
        let result = WebhookClientBuilder::new("http://example.com/webhook").build();
        assert!(matches!(result, Err(WebhookError::InvalidUrl { .. })));
    }
}
