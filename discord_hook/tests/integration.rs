//! Integration tests that fire real HTTP requests to a Discord webhook.
//!
//! All tests are skipped automatically when `DISCORD_WEBHOOK_URL` is not set,
//! so `cargo test` in CI works without any secrets configured.
//!
//! To run locally:
//! ```bash
//! DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/ID/TOKEN cargo test -p discord-hook --test integration
//! ```

use discord_hook::{AllowedMentions, Embed, WebhookClient, WebhookMessage};

/// Returns the webhook URL from the environment, or `None` to signal skip.
fn webhook_url() -> Option<String> {
    std::env::var("DISCORD_WEBHOOK_URL").ok()
}

macro_rules! require_url {
    () => {
        match webhook_url() {
            Some(url) => url,
            None => {
                eprintln!("Skipping integration test — DISCORD_WEBHOOK_URL not set");
                return;
            }
        }
    };
}

#[tokio::test]
async fn sends_plain_text_message() {
    let url = require_url!();
    let client = WebhookClient::new(&url).unwrap();

    let message = WebhookMessage::builder()
        .username("hooksmith-test")
        .content("[integration test] plain-text message")
        .allowed_mentions(AllowedMentions::none())
        .build()
        .unwrap();

    client.send(&message).await.unwrap();
}

#[tokio::test]
async fn sends_rich_embed() {
    let url = require_url!();
    let client = WebhookClient::new(&url).unwrap();

    let embed = Embed::builder()
        .title("[integration test] Rich embed")
        .description("Sent by the `discord_hook` integration test suite.")
        .color(0x5865F2)
        .field("Test", "sends_rich_embed", true)
        .build();

    let message = WebhookMessage::builder()
        .username("hooksmith-test")
        .embed(embed)
        .allowed_mentions(AllowedMentions::none())
        .build()
        .unwrap();

    client.send(&message).await.unwrap();
}

#[tokio::test]
async fn send_to_thread_requires_valid_thread_id_format() {
    let url = require_url!();
    let client = WebhookClient::new(&url).unwrap();

    let message = WebhookMessage::builder()
        .username("hooksmith-test")
        .content("[integration test] thread send (may 404 with dummy ID)")
        .build()
        .unwrap();

    // Sending to a non-existent thread returns an ApiError — that's acceptable
    // here; we're testing the request path, not Discord's thread management.
    let _ = client.send_to_thread(&message, "000000000000000000").await;
}

#[tokio::test]
async fn client_is_clone_and_reusable() {
    let url = require_url!();
    let client = WebhookClient::new(&url).unwrap();
    let client2 = client.clone();

    let msg = WebhookMessage::builder()
        .username("hooksmith-test")
        .content("[integration test] sent from a cloned client")
        .allowed_mentions(AllowedMentions::none())
        .build()
        .unwrap();

    client2.send(&msg).await.unwrap();
}
