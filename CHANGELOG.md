<!-- @format -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

## [0.1.2] - 2026-04-22

### Added

#### `discord_hook`

- `WebhookClient` now derives `Clone` — share a client across Axum state or other async contexts without needing `Arc`.
- `WebhookClient::send_with_retry(&self, message, &RetryPolicy)` — sends with automatic exponential-backoff retries on transport failure. Discord 429 responses are surfaced immediately as `WebhookError::RateLimited` so callers can respect `retry_after_ms`.
- Three runnable examples in `examples/`: `basic_message`, `rich_embed`, `send_with_retry`.
- Integration test suite in `tests/integration.rs` — all tests skip automatically when `DISCORD_WEBHOOK_URL` is not set.

#### `hooksmith-core`

- `HttpClient` now derives `Clone`.

#### Repository

- GitHub Actions CI workflow (`.github/workflows/ci.yml`): `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` on every push and pull request.
- This changelog.

---

## [0.1.1] - 2026-04-01

### Added

#### `hooksmith-core`

- `RetryPolicy` and `HttpClient::post_json_with_retry` — exponential backoff with optional jitter.
- `CoreError` — shared transport-level error type wrapping `reqwest::Error` and `serde_json::Error`.
- `MockSender<M>` behind the `mock` Cargo feature — captures sent messages for unit-test assertions.
- Optional `tracing` feature: `info_span!("hooksmith.post_json", ...)` around every request.
- `WebhookSender::send_batch` default method — fans out a slice of messages sequentially.

#### `discord_hook`

- `AllowedMentions` and `AllowedMentionType` for controlling which `@mentions` notify.
- `flags` module: `SUPPRESS_EMBEDS`, `SUPPRESS_NOTIFICATIONS`, `IS_COMPONENTS_V2`.
- `WebhookMessage` fields: `tts`, `allowed_mentions`, `flags`, `thread_name`, `applied_tags`.
- `WebhookClient::send_to_thread` — post into an existing thread by snowflake ID.
- `EmbedBuilder::json_description` and `EmbedBuilder::json_field` — pretty-print serializable values as JSON code blocks.
- `EmbedBuilder::author_full` and `EmbedBuilder::footer_with_icon`.

---

## [0.1.0] - 2026-03-15

### Added

- Initial release of `hooksmith-core` and `discord_hook`.
- `WebhookClient` with URL validation (HTTPS + `discord.com/api/webhooks/` check), `SecretString` storage, and `WebhookSender` trait implementation.
- `Embed` and `EmbedBuilder` with title, description, URL, color, footer, thumbnail, image, author, fields, and timestamp.
- `WebhookMessage` and `WebhookMessageBuilder`.
- `WebhookError` with `InvalidUrl`, `Http`, `Json`, `RateLimited`, and `ApiError` variants.
- `rustls` (default) and `native-tls` feature flags for TLS backend selection.
- 30-second request timeout on the default `HttpClient`.
