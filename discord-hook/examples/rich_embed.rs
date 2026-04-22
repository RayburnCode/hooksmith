//! Send a rich embed to a Discord webhook.
//!
//! ```bash
//! DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/ID/TOKEN \
//!     cargo run -p discord-hook --example rich_embed
//! ```

use discord_hook::{AllowedMentions, Embed, WebhookClient, WebhookMessage};

#[tokio::main]
async fn main() -> Result<(), discord_hook::WebhookError> {
    let url = std::env::var("DISCORD_WEBHOOK_URL")
        .expect("Set DISCORD_WEBHOOK_URL to your webhook URL");

    let client = WebhookClient::new(&url)?;

    let embed = Embed::builder()
        .title("hooksmith — rich embed demo")
        .description("This embed was sent from the `discord_hook` crate.")
        .color(0x5865F2) // Discord blurple
        .field("Crate", "discord_hook", true)
        .field("Version", env!("CARGO_PKG_VERSION"), true)
        .field("Async runtime", "Tokio", true)
        .footer("hooksmith")
        .build();

    let message = WebhookMessage::builder()
        .username("hooksmith")
        .embed(embed)
        .allowed_mentions(AllowedMentions::none())
        .build()?;

    client.send(&message).await?;
    println!("Embed sent.");
    Ok(())
}
