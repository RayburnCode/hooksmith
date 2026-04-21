<!-- @format -->

# `discord_hook` — Integration Guide

This document is written so that any AI assistant (or human developer) can
understand what this crate does and how to wire it into a Rust web project
from scratch.

---

## What this crate does

`discord_hook` is an async Rust library that sends messages to Discord via
**incoming webhooks**. It supports:

- Plain text content
- Rich **embeds** (title, description, fields, colour, footer, author, image,
  thumbnail, timestamp)
- Per-message bot username and avatar override
- Thread-targeted sends
- `allowed_mentions` control (prevent accidental `@everyone` pings)
- Message flags (`SUPPRESS_NOTIFICATIONS`, `SUPPRESS_EMBEDS`, etc.)
- `?wait=true` by default — Discord confirms the message was saved, so API
  errors are never silently dropped
- Configurable TLS backend (`rustls` default, or `native-tls`)

---

## 1. Add the dependency

### From crates.io (recommended for production)

```toml
# Cargo.toml of your project
[dependencies]
discord_hook = "0.1"
```

### From a local path (monorepo / development)

```toml
[dependencies]
discord_hook = { path = "../hooksmith/discord_hook" }
```

### TLS feature flags

| Feature      | Default? | Notes                                   |
| ------------ | -------- | --------------------------------------- |
| `rustls`     | ✅ yes   | Pure-Rust TLS, no system OpenSSL needed |
| `native-tls` | ❌ no    | Links against the OS TLS library        |

To use `native-tls` instead:

```toml
discord_hook = { version = "0.1", default-features = false, features = ["native-tls"] }
```

---

## 2. Get a Discord webhook URL

1. Open Discord and go to the target channel.
2. **Edit Channel → Integrations → Webhooks → New Webhook**
3. Click **Copy Webhook URL**.

The URL looks like:

```
https://discord.com/api/webhooks/1234567890123456789/xXxXxXxX...
```

**Never hard-code this URL.** Store it in an environment variable and read it
at startup (see §3).

---

## 3. Store the URL securely

```bash
# .env  — add to .gitignore, never commit
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/YOUR_ID/YOUR_TOKEN
```

At runtime, read it with `std::env::var`:

```rust
let url = std::env::var("DISCORD_WEBHOOK_URL")
    .expect("DISCORD_WEBHOOK_URL must be set");
```

For production servers, set the variable in your hosting environment (e.g.
Railway, Fly.io, AWS, Docker `--env-file`).

---

## 4. Core API overview

### `WebhookClient`

```rust
use discord_hook::WebhookClient;

// Validates that the URL is HTTPS + targets discord.com/api/webhooks/
let client = WebhookClient::new("https://discord.com/api/webhooks/ID/TOKEN")?;

// Reuse an existing reqwest::Client (connection pool sharing)
let reqwest_client = reqwest::Client::new();
let client = WebhookClient::with_client("https://...", reqwest_client)?;
```

### `WebhookMessage`

```rust
use discord_hook::{WebhookMessage, Embed};

let message = WebhookMessage::builder()
    .content("Hello from my app!")   // plain text (optional)
    .username("MyBot")                // override the webhook's default name
    .avatar_url("https://example.com/avatar.png")
    .embed(
        Embed::builder()
            .title("Something happened")
            .description("More detail here")
            .color(0x5865F2)          // hex colour int
            .field("Key", "Value", true)   // inline field
            .field("Another", "Field", false)
            .footer("My App")
            .timestamp("2025-01-01T00:00:00.000Z") // ISO 8601
            .build(),
    )
    .build()?;

client.send(&message).await?;
```

### `Embed` builder — all options

```rust
Embed::builder()
    .title("Title")
    .description("Body text, supports **markdown**")
    .url("https://example.com")          // makes title a hyperlink
    .color(0xFF5733)
    .footer("Footer text")
    .footer_with_icon("Footer", "https://example.com/icon.png")
    .thumbnail("https://example.com/thumb.png")
    .image("https://example.com/big-image.png")
    .author("Author name")
    .author_full("Author", Some("https://url"), Some("https://icon"))
    .field("Name", "Value", true)        // inline
    .field("Name", "Value", false)       // non-inline
    .timestamp("2025-06-01T12:00:00Z")
    .build()
```

### Sending to a thread

```rust
client.send_to_thread(&message, "THREAD_SNOWFLAKE_ID").await?;
```

### Suppressing notifications (silent alert)

```rust
use discord_hook::flags;

let message = WebhookMessage::builder()
    .content("Background job finished")
    .flag(flags::SUPPRESS_NOTIFICATIONS)
    .build()?;
```

### Preventing accidental pings (user-generated content)

```rust
use discord_hook::AllowedMentions;

let message = WebhookMessage::builder()
    .content(user_supplied_text)
    .allowed_mentions(AllowedMentions::none())  // no @everyone, no @role, no @user
    .build()?;
```

---

## 5. Error handling

`discord_hook` uses a single `WebhookError` enum:

| Variant                          | When it occurs                                 |
| -------------------------------- | ---------------------------------------------- |
| `InvalidUrl`                     | URL is not HTTPS or doesn't target discord.com |
| `EmptyMessage`                   | Message has neither `content` nor any embeds   |
| `Http(reqwest::Error)`           | Network/transport failure                      |
| `RateLimited { retry_after_ms }` | Discord returned HTTP 429                      |
| `ApiError { status, message }`   | Any other non-2xx Discord response             |
| `Json(serde_json::Error)`        | Serialization failure (rare)                   |

Handling rate limits:

```rust
use discord_hook::WebhookError;
use std::time::Duration;
use tokio::time::sleep;

match client.send(&message).await {
    Ok(()) => {}
    Err(WebhookError::RateLimited { retry_after_ms }) => {
        sleep(Duration::from_millis(retry_after_ms)).await;
        client.send(&message).await?;
    }
    Err(e) => return Err(e.into()),
}
```

---

## 6. Integration patterns by framework

### 6a. Axum web server

A typical pattern: create the client once at startup, share it via
`axum::extract::State`.

```toml
# Cargo.toml
[dependencies]
discord_hook = "0.1"
axum = "0.8"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

```rust
// src/main.rs
use std::sync::Arc;
use axum::{Router, extract::State, routing::post, Json, http::StatusCode};
use discord_hook::{WebhookClient, WebhookMessage, Embed};
use serde::Deserialize;

#[derive(Clone)]
struct AppState {
    discord: Arc<WebhookClient>,
}

#[derive(Deserialize)]
struct ContactForm {
    name: String,
    email: String,
    message: String,
}

async fn handle_contact(
    State(state): State<AppState>,
    Json(form): Json<ContactForm>,
) -> StatusCode {
    let embed = Embed::builder()
        .title("New Contact Form Submission")
        .color(0x57F287) // green
        .field("Name", &form.name, true)
        .field("Email", &form.email, true)
        .field("Message", &form.message, false)
        .build();

    let message = WebhookMessage::builder()
        .username("Contact Bot")
        .embed(embed)
        .build()
        .unwrap();

    match state.discord.send(&message).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[tokio::main]
async fn main() {
    let url = std::env::var("DISCORD_WEBHOOK_URL").expect("DISCORD_WEBHOOK_URL not set");
    let discord = Arc::new(WebhookClient::new(&url).expect("invalid webhook URL"));

    let state = AppState { discord };

    let app = Router::new()
        .route("/contact", post(handle_contact))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

---

### 6b. Actix-web

```toml
[dependencies]
discord_hook = "0.1"
actix-web = "4"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

```rust
use std::sync::Arc;
use actix_web::{web, App, HttpServer, HttpResponse, post};
use discord_hook::{WebhookClient, WebhookMessage, Embed};
use serde::Deserialize;

struct AppData {
    discord: Arc<WebhookClient>,
}

#[derive(Deserialize)]
struct AlertPayload {
    title: String,
    body: String,
}

#[post("/alert")]
async fn send_alert(
    data: web::Data<AppData>,
    payload: web::Json<AlertPayload>,
) -> HttpResponse {
    let embed = Embed::builder()
        .title(&payload.title)
        .description(&payload.body)
        .color(0xED4245) // red
        .build();

    let message = WebhookMessage::builder()
        .embed(embed)
        .build()
        .unwrap();

    match data.discord.send(&message).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let url = std::env::var("DISCORD_WEBHOOK_URL").expect("DISCORD_WEBHOOK_URL not set");
    let discord = Arc::new(WebhookClient::new(&url).expect("invalid webhook URL"));

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppData { discord: discord.clone() }))
            .service(send_alert)
    })
    .bind("0.0.0.0:3000")?
    .run()
    .await
}
```

---

### 6c. Dioxus fullstack (WASM + server)

The webhook URL stays server-side only. The browser calls a typed `#[server]`
function over HTTP.

```toml
[dependencies]
discord_hook = "0.1"
dioxus = { version = "0.6", features = ["fullstack"] }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

```rust
// Server function — only compiled/executed on the server
#[server]
pub async fn notify_lead(name: String, email: String, msg: String) -> Result<(), ServerFnError> {
    let url = std::env::var("DISCORD_WEBHOOK_URL")
        .map_err(|_| ServerFnError::new("DISCORD_WEBHOOK_URL not set"))?;

    let client = discord_hook::WebhookClient::new(&url)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let embed = discord_hook::Embed::builder()
        .title("New Lead")
        .field("Name", &name, true)
        .field("Email", &email, true)
        .field("Message", &msg, false)
        .color(0x5865F2)
        .build();

    let message = discord_hook::WebhookMessage::builder()
        .username("Lead Bot")
        .embed(embed)
        .build()
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    client.send(&message).await.map_err(|e| ServerFnError::new(e.to_string()))
}
```

---

### 6d. Standalone / background worker (no web framework)

```rust
use discord_hook::{WebhookClient, WebhookMessage, Embed};

#[tokio::main]
async fn main() -> Result<(), discord_hook::WebhookError> {
    let client = WebhookClient::new(
        &std::env::var("DISCORD_WEBHOOK_URL").expect("DISCORD_WEBHOOK_URL not set"),
    )?;

    let message = WebhookMessage::builder()
        .username("Deploy Bot")
        .embed(
            Embed::builder()
                .title("Deployment complete")
                .description("Version 1.2.3 is live on production.")
                .color(0x57F287)
                .build(),
        )
        .build()?;

    client.send(&message).await
}
```

---

## 7. Sharing one `reqwest::Client` across the app

`reqwest::Client` manages a connection pool; creating many of them wastes
resources. Use `WebhookClient::with_client` to inject a shared one:

```rust
let http = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(10))
    .build()
    .unwrap();

let discord = WebhookClient::with_client("https://discord.com/api/webhooks/...", http)?;
```

---

## 8. Publishing checklist (crates.io)

1. **Log in**

   ```bash
   cargo login          # opens crates.io in browser, paste your API token
   ```

2. **Dry-run from the crate directory**

   ```bash
   cd discord_hook
   cargo publish --dry-run
   ```

   Fix any warnings or errors before proceeding.

3. **Check the name is available**
   Search [crates.io](https://crates.io) for `discord_hook`. If it's taken,
   update the `name` field in `Cargo.toml` (e.g. `discord-hooksmith`) and
   update all `use discord_hook::` imports in tests and examples.

4. **Publish**

   ```bash
   cargo publish
   ```

5. **Verify** — your crate page will be live at
   `https://crates.io/crates/discord_hook` within a minute.
