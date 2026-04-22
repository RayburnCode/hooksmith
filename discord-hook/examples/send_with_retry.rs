//! Send a message using automatic retry on transport failure.
//!
//! ```bash
//! DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/ID/TOKEN \
//!     cargo run -p discord_hook --example send_with_retry
//! ```

use discord_hook::{WebhookClient, WebhookMessage};
use hooksmith_core::RetryPolicy;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), discord_hook::WebhookError> {
    let url = std::env::var("DISCORD_WEBHOOK_URL")
        .expect("Set DISCORD_WEBHOOK_URL to your webhook URL");

    let client = WebhookClient::new(&url)?;

    let policy = RetryPolicy {
        max_attempts: 4,
        base_delay: Duration::from_millis(250),
        jitter: true,
    };

    let message = WebhookMessage::builder()
        .username("hooksmith")
        .content("Sent with automatic retry on transport failure.")
        .build()?;

    client.send_with_retry(&message, &policy).await?;
    println!("Message sent (with retry policy).");
    Ok(())
}
