<!-- @format -->

# hooksmith

# 1. Log in (one-time — opens browser to get your API token)

cargo login

# 2. Dry-run from inside the crate folder

cd discord_hook
cargo publish --dry-run

# 3. Actually publish

cargo publish

https://docs.discord.com/developers/resources/webhook
A collection of Rust webhook crates for sending notifications to external services.

| Crate                            | Description                                |
| -------------------------------- | ------------------------------------------ |
| [`discord_hook`](./discord_hook) | Send rich messages to Discord via webhooks |

---

## `discord_hook` — Dioxus Lead Form Integration Guide

This guide shows how to wire `discord_hook` into a **Rust + Dioxus** website so that every lead-form submission fires a Discord notification.

### How it works

```
Browser (Dioxus WASM)
  └─ calls  ──►  #[server] fn submit_lead(...)   (runs on your server)
                    └─ builds WebhookMessage
                    └─ calls WebhookClient::send(...)
                          └─ HTTPS POST ──►  Discord webhook URL
```

The webhook URL (which contains your Discord token) **never leaves the server**. The browser only calls your typed server function.

---

### 1. Add the dependency

In your website's `Cargo.toml`:

```toml
[dependencies]
discord_hook = { path = "../hooksmith/discord_hook" }   # or a crates.io version once published
dioxus = { version = "0.6", features = ["fullstack"] }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

---

### 2. Store the webhook URL securely

Never hard-code the URL. Read it from an environment variable at startup.

```bash
# .env  (add to .gitignore — never commit this file)
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/YOUR_ID/YOUR_TOKEN
```

> **Getting a webhook URL:** In Discord, open a channel → Edit Channel → Integrations → Webhooks → New Webhook → Copy Webhook URL.

---

### 3. Server function

Create a server function that the Dioxus component calls. It runs **only on the server** — the client never sees the webhook URL.

```rust
// src/server/notifications.rs

use dioxus::prelude::*;
use discord_hook::{Embed, WebhookClient, WebhookMessage};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LeadFormData {
    pub name: String,
    pub email: String,
    pub message: String,
}

/// Called by the Dioxus component when the lead form is submitted.
/// Runs on the server — the webhook URL never reaches the browser.
#[server]
pub async fn notify_lead(lead: LeadFormData) -> Result<(), ServerFnError> {
    let url = std::env::var("DISCORD_WEBHOOK_URL")
        .map_err(|_| ServerFnError::new("DISCORD_WEBHOOK_URL is not set"))?;

    let client = WebhookClient::new(&url)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let embed = Embed::builder()
        .title("New Lead")
        .color(0x5865F2) // Discord blurple
        .field("Name", &lead.name, true)
        .field("Email", &lead.email, true)
        .field("Message", &lead.message, false)
        .footer("Your Website")
        .build();

    let message = WebhookMessage::builder()
        .username("Lead Bot")
        .embed(embed)
        .build()
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    client
        .send(&message)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}
```

---

### 4. Dioxus component

```rust
// src/components/lead_form.rs

use dioxus::prelude::*;
use crate::server::notifications::{notify_lead, LeadFormData};

#[component]
pub fn LeadForm() -> Element {
    let mut name = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut message = use_signal(String::new);
    let mut status = use_signal(|| Option::<String>::None);
    let mut submitting = use_signal(|| false);

    let on_submit = move |evt: FormEvent| {
        evt.prevent_default();
        let lead = LeadFormData {
            name: name.read().clone(),
            email: email.read().clone(),
            message: message.read().clone(),
        };

        spawn(async move {
            submitting.set(true);
            status.set(None);

            match notify_lead(lead).await {
                Ok(_) => {
                    status.set(Some("Thanks! We'll be in touch soon.".into()));
                    name.set(String::new());
                    email.set(String::new());
                    message.set(String::new());
                }
                Err(e) => {
                    status.set(Some(format!("Something went wrong: {e}")));
                }
            }

            submitting.set(false);
        });
    };

    rsx! {
        form { onsubmit: on_submit,
            input {
                r#type: "text",
                placeholder: "Your name",
                value: "{name}",
                oninput: move |e| name.set(e.value()),
                required: true,
            }
            input {
                r#type: "email",
                placeholder: "your@email.com",
                value: "{email}",
                oninput: move |e| email.set(e.value()),
                required: true,
            }
            textarea {
                placeholder: "How can we help?",
                value: "{message}",
                oninput: move |e| message.set(e.value()),
                required: true,
            }
            button {
                r#type: "submit",
                disabled: *submitting.read(),
                if *submitting.read() { "Sending…" } else { "Send Message" }
            }
        }

        if let Some(msg) = status.read().as_deref() {
            p { "{msg}" }
        }
    }
}
```

---

### 5. What the Discord notification looks like

When a lead submits the form, your Discord channel receives a message like:

```
Lead Bot  [BOT]
┌─────────────────────────────────┐
│ New Lead                        │
│                                 │
│ Name           Email            │
│ Jane Smith     jane@example.com │
│                                 │
│ Message                         │
│ I'd love to learn more about…   │
│                                 │
│ Your Website                    │
└─────────────────────────────────┘
```

---

### Security notes

| Concern                    | How it's handled                                                                   |
| -------------------------- | ---------------------------------------------------------------------------------- |
| Webhook URL (secret token) | Stored in env var, only read server-side inside `#[server]` fn                     |
| HTTPS enforcement          | `WebhookClient::new` rejects any non-HTTPS or non-Discord URL at construction time |
| TLS                        | `discord_hook` defaults to `rustls` (pure-Rust TLS, no OpenSSL dependency)         |
| User input                 | Passed as typed Rust struct — no raw string concatenation into the payload         |
| `.env` file                | Add to `.gitignore`; use your host's secret management in production               |
