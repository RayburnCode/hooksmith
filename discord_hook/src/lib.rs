//! `discord_hook` — Send messages to Discord via webhooks.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use discord_hook::{WebhookClient, WebhookMessage, Embed};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), discord_hook::WebhookError> {
//!     let client = WebhookClient::new("https://discord.com/api/webhooks/ID/TOKEN");
//!
//!     let message = WebhookMessage::builder()
//!         .content("Hello from discord_hook!")
//!         .username("MyBot")
//!         .embed(
//!             Embed::builder()
//!                 .title("Rich embed")
//!                 .description("Supports titles, fields, colours, and more.")
//!                 .color(0x5865F2)   // Discord blurple
//!                 .field("Library", "discord_hook", true)
//!                 .build(),
//!         )
//!         .build()?;
//!
//!     client.send(&message).await
//! }
//! ```
//!
//! # Rate limits
//!
//! When Discord returns HTTP 429 the client surfaces a
//! [`WebhookError::RateLimited`] error that contains `retry_after_ms`.
//! Use that value to back off before retrying.

pub mod client;
pub mod error;
pub mod message;

pub use client::WebhookClient;
pub use error::WebhookError;
pub use hooksmith_core::WebhookSender;
pub use message::{
    flags, json_code_block, AllowedMentionType, AllowedMentions, Embed, EmbedAuthor, EmbedBuilder,
    EmbedField, EmbedFooter, EmbedImage, EmbedThumbnail, WebhookMessage, WebhookMessageBuilder,
};
