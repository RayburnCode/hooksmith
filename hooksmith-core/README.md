<!-- @format -->

# hooksmith-core

Shared building blocks for the hooksmith family of webhook crates.

This crate is not a webhook client itself. It provides the common trait, HTTP
client, error type, and utilities that every `*_hook` service crate builds on.

---

## What's inside

### `WebhookSender` trait

The single abstraction every service crate implements. Application code can be
generic over the backend:

```rust
use hooksmith_core::WebhookSender;

async fn notify<S>(sender: &S, msg: &S::Message) -> Result<(), S::Error>
where
    S: WebhookSender,
{
    sender.send(msg).await
}
```

#### `send_batch`

A default method that fans out a slice of messages concurrently using
[`futures::future::join_all`]. One failure does not abort the others — you
get back one `Result` per message.

```rust
let results = sender.send_batch(&[&msg_a, &msg_b, &msg_c]).await;
for result in results {
    result?;
}
```

---

### `HttpClient`

A thin wrapper around [`reqwest::Client`] with a sensible 30-second default
timeout. Every `*_hook` crate holds one and calls `post_json` to fire requests.

```rust
use hooksmith_core::HttpClient;

let client = HttpClient::new();
let resp = client.post_json("https://hooks.example.com/...", &payload).await?;
```

Pass your own `reqwest::Client` to share a connection pool or customise TLS:

```rust
let inner = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(10))
    .build()?;
let client = HttpClient::with_reqwest(inner);
```

#### `post_json_with_retry`

Retries failed requests with **exponential backoff** according to a
[`RetryPolicy`]. Pair it with the tracing feature to get per-attempt
observability at no extra effort.

```rust
use hooksmith_core::{HttpClient, RetryPolicy};
use std::time::Duration;

let policy = RetryPolicy {
    max_attempts: 4,
    base_delay:   Duration::from_millis(250),
    jitter:       true,
};

let resp = client.post_json_with_retry(url, &payload, &policy).await?;
```

---

### `RetryPolicy`

Controls how `post_json_with_retry` behaves.

| Field          | Type       | Default  | Description                                         |
| -------------- | ---------- | -------- | --------------------------------------------------- |
| `max_attempts` | `u32`      | `3`      | Total tries including the first (clamped to ≥ 1)    |
| `base_delay`   | `Duration` | `500 ms` | Delay before the first retry (doubles each time)    |
| `jitter`       | `bool`     | `true`   | Add a random sub-delay to spread concurrent retries |

```rust
// Use defaults
let policy = RetryPolicy::default();

// Customise
let policy = RetryPolicy { max_attempts: 5, ..Default::default() };
```

---

### `CoreError`

A transport-level error enum that service-specific errors should wrap via
`#[from]`, so generic code can match on network or JSON failures without
knowing which service is in use.

```rust
#[derive(Debug, thiserror::Error)]
pub enum DiscordError {
    #[error(transparent)]
    Core(#[from] hooksmith_core::CoreError),

    #[error("Discord API error {status}: {body}")]
    Api { status: u16, body: String },
}
```

Variants:

| Variant   | Wraps               | When                             |
| --------- | ------------------- | -------------------------------- |
| `Network` | `reqwest::Error`    | HTTP or connection-level failure |
| `Json`    | `serde_json::Error` | Serialisation / deserialisation  |

---

## Feature flags

| Flag      | What it enables                                                            |
| --------- | -------------------------------------------------------------------------- |
| `mock`    | Exposes `hooksmith_core::mock::MockSender` for use in tests                |
| `tracing` | Emits `tracing` spans around every `post_json` call (URL, status, latency) |

### `mock` — `MockSender`

A `WebhookSender` that captures messages in memory instead of hitting a real
endpoint. Add it to `dev-dependencies` so it is only compiled for tests:

```toml
[dev-dependencies]
hooksmith-core = { version = "*", features = ["mock"] }
```

```rust
use hooksmith_core::mock::MockSender;

let sender: MockSender<MyMessage> = MockSender::new();
sender.send(&my_message).await.unwrap();

assert_eq!(sender.len(), 1);
assert_eq!(sender.messages()[0], my_message);
```

### `tracing` — observability spans

Enable the `tracing` feature to have every `post_json` call automatically
instrumented with an `info_span` named `hooksmith.post_json`. The span records:

- `url` — the request URL
- `status` — HTTP status code (on success)
- `latency_ms` — wall-clock time for the request
- `error` — error string (on failure)

```toml
[dependencies]
hooksmith-core = { version = "*", features = ["tracing"] }
```

No other code changes are required — wire up your `tracing` subscriber as
normal and the spans appear automatically.

---

## Using hooksmith-core in a service crate

1. Add it to your `Cargo.toml`:

   ```toml
   [dependencies]
   hooksmith-core = { version = "*" }
   ```

2. Implement `WebhookSender`:

   ```rust
   use hooksmith_core::{HttpClient, WebhookSender, CoreError};

   pub struct MyClient {
       http: HttpClient,
       url:  String,
   }

   impl WebhookSender for MyClient {
       type Message = MyMessage;
       type Error   = MyError; // wraps CoreError

       fn send(&self, msg: &MyMessage) -> impl Future<Output = Result<(), MyError>> + Send {
           async move {
               self.http.post_json(&self.url, msg).await
                   .map_err(CoreError::from)
                   .map_err(MyError::Core)?;
               Ok(())
           }
       }
   }
   ```

3. `send_batch` is inherited for free from the default trait implementation.
