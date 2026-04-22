//! Send a plain-text message to a Discord webhook.
//!
//! ```bash
//! DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/ID/TOKEN \
//!     cargo run -p discord_hook --example basic_message
//! ```

use discord_hook::{WebhookClient, WebhookMessage};

#[tokio::main]
async fn main() -> Result<(), discord_hook::WebhookError> {
    let url =
        std::env::var("DISCORD_WEBHOOK_URL").expect("Set DISCORD_WEBHOOK_URL to your webhook URL");

    let client = WebhookClient::new(&url)?;

    let message = WebhookMessage::builder()
        .username("hooksmith")
        .content("Hello from discord_hook! 👋")
        .build()?;

    client.send(&message).await?;
    println!("Message sent.");
    Ok(())
}
