use reqwest::Client;

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
    url: String,
    client: Client,
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
        Ok(Self { url, client: Client::new() })
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
        Ok(Self { url, client })
    }

    /// Send a [`WebhookMessage`] to Discord.
    ///
    /// # Errors
    ///
    /// - [`WebhookError::Http`] – transport-level failure.
    /// - [`WebhookError::RateLimited`] – Discord returned HTTP 429; the
    ///   `retry_after_ms` field tells you how long to wait.
    /// - [`WebhookError::ApiError`] – any other non-2xx response.
    pub async fn send(&self, message: &WebhookMessage) -> Result<(), WebhookError> {
        let response = self
            .client
            .post(&self.url)
            .json(message)
            .send()
            .await?;

        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            // Discord may send the retry delay in seconds as a float.
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
