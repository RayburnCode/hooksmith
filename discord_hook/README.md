<!-- @format -->

# discord_hook

An async Rust crate for sending rich messages to Discord via webhooks.

[![Crates.io](https://img.shields.io/crates/v/discord_hook)](https://crates.io/crates/discord_hook)
[![Docs.rs](https://docs.rs/discord_hook/badge.svg)](https://docs.rs/discord_hook)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)

---

## Features

- **Typed message builder** — construct `WebhookMessage` and `Embed` payloads with a fluent builder API; validation happens at `.build()` time, not at runtime.
- **`discord_message!` / `embed!` macros** — shorthand for common one-liner messages.
- **Rate-limit awareness** — Discord's HTTP 429 surfaces as `WebhookError::RateLimited` with a `retry_after_ms` field so you can back off correctly.
- **Thread support** — post into existing threads or create new ones in forum channels.
- **`AllowedMentions` control** — always safe to pass user-generated content; silence `@everyone` pings with a single call.
- **JSON code blocks** — format any `Serialize` value as a Discord code block in message content or embed descriptions/fields.
- **Pluggable TLS** — `rustls` by default (pure Rust, no OpenSSL); opt into `native-tls` via a feature flag.
- **`WebhookSender` trait** — write code that is generic over notification backends (Discord, Slack, etc.) via `hooksmith-core`.

---

## Installation

```toml
[dependencies]
discord_hook = "0.1"
tokio = { version = "1", features = ["full"] }
```

### TLS backends

| Feature      | Default | Notes                                                         |
| ------------ | ------- | ------------------------------------------------------------- |
| `rustls`     | ✅      | Pure Rust — no system OpenSSL required                        |
| `native-tls` | ❌      | Uses the OS TLS stack (OpenSSL / SChannel / Secure Transport) |

To switch to `native-tls`:

```toml
discord_hook = { version = "0.1", default-features = false, features = ["native-tls"] }
```

---

## Quick start

```rust,no_run
use discord_hook::{WebhookClient, WebhookMessage, Embed};

#[tokio::main]
async fn main() -> Result<(), discord_hook::WebhookError> {
    let client = WebhookClient::new("https://discord.com/api/webhooks/ID/TOKEN")?;

    let message = WebhookMessage::builder()
        .username("MyBot")
        .embed(
            Embed::builder()
                .title("Deployment succeeded")
                .description("Branch `main` is live.")
                .color(0x57F287) // green
                .field("Environment", "production", true)
                .field("Duration", "42s", true)
                .build(),
        )
        .build()?;

    client.send(&message).await
}
```

---

## Macros

For simple messages the `discord_message!` and `embed!` macros remove boilerplate:

```rust,no_run
use discord_hook::{discord_message, embed, flags};

# #[tokio::main]
# async fn main() -> Result<(), discord_hook::WebhookError> {
# let client = discord_hook::WebhookClient::new("https://discord.com/api/webhooks/1/t")?;
// Plain text message
let msg = discord_message!(content = "Deploy started", username = "CI Bot").unwrap();

// Message with a quick embed
let msg = discord_hook::WebhookMessage::builder()
    .embed(embed!(title = "Alert", color = 0xED4245u32)) // red
    .build()?;

// Silent notification
let msg = discord_message!(
    content = "Background job finished",
    flag    = flags::SUPPRESS_NOTIFICATIONS,
).unwrap();
# Ok(())
# }
```

---

## Sending to a thread

```rust,no_run
# use discord_hook::{WebhookClient, WebhookMessage};
# #[tokio::main]
# async fn main() -> Result<(), discord_hook::WebhookError> {
# let client = WebhookClient::new("https://discord.com/api/webhooks/1/t")?;
let msg = WebhookMessage::builder().content("Update posted.").build()?;

// Existing thread
client.send_to_thread(&msg, "1234567890123456789").await?;
# Ok(())
# }
```

---

## Safe handling of user-generated content

Always suppress auto-parsed mentions when the message body contains input you
do not fully control:

```rust,no_run
# use discord_hook::{WebhookClient, WebhookMessage, AllowedMentions};
# #[tokio::main]
# async fn main() -> Result<(), discord_hook::WebhookError> {
# let client = WebhookClient::new("https://discord.com/api/webhooks/1/t")?;
# let user_input = String::from("hello @everyone");
let msg = WebhookMessage::builder()
    .content(user_input)
    .allowed_mentions(AllowedMentions::none()) // ← prevents @everyone / role pings
    .build()?;

client.send(&msg).await
# }
```

---

## JSON payloads in embeds

Any `serde::Serialize` value can be rendered as a pretty code block:

```rust
use discord_hook::Embed;
use serde::Serialize;

#[derive(Serialize)]
struct Event { kind: String, status: u16 }

let embed = Embed::builder()
    .title("Webhook received")
    .json_description(&Event { kind: "push".into(), status: 200 })
    .expect("serialization failed")
    .build();
```

---

## Rate limits

Discord can return HTTP 429. The client surfaces this as:

```rust,ignore
WebhookError::RateLimited { retry_after_ms: u64 }
```

Back off and retry after that many milliseconds. For automatic retry with
exponential backoff use `HttpClient::post_json_with_retry` from `hooksmith-core`.

---

## Error handling

| Variant                      | When                                                           |
| ---------------------------- | -------------------------------------------------------------- |
| `WebhookError::InvalidUrl`   | URL is not HTTPS or doesn't target `discord.com/api/webhooks/` |
| `WebhookError::EmptyMessage` | `.build()` called with no content and no embeds                |
| `WebhookError::Http`         | Transport-level `reqwest` failure                              |
| `WebhookError::RateLimited`  | Discord returned HTTP 429                                      |
| `WebhookError::ApiError`     | Any other non-2xx response from Discord                        |
| `WebhookError::Json`         | JSON serialization failed                                      |

---

## Writing backend-agnostic code

`discord_hook` re-exports `WebhookSender` from `hooksmith-core`. Implement it
once and swap backends without changing call sites:

```rust,ignore
use discord_hook::WebhookSender;

async fn notify<S: WebhookSender>(sender: &S, msg: &S::Message) -> Result<(), S::Error> {
    sender.send(msg).await
}
```

---

## License

MIT — see [LICENSE](../LICENSE).
