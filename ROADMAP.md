<!-- @format -->

# hooksmith — Future Roadmap

A running list of ideas and improvements, roughly ordered by priority.

---

## New service crates

- [ ] **`slack_hook`** — Slack incoming webhooks. Supports Block Kit (sections, dividers, images, buttons). URL pattern: `hooks.slack.com/services/...`
- [ ] **`gchat_hook`** — Google Chat spaces webhooks. Supports cards v2 (key-value pairs, images, buttons). URL pattern: `chat.googleapis.com/v1/spaces/.../messages`
- [ ] **`teams_hook`** — Microsoft Teams incoming webhooks. Supports Adaptive Cards. URL pattern: `*.webhook.office.com/webhookb2/...`
- [ ] **`pagerduty_hook`** — PagerDuty Events API v2. Trigger, acknowledge, and resolve incidents from app code.
- [ ] **`ntfy_hook`** — [ntfy.sh](https://ntfy.sh) self-hosted push notifications. Dead-simple HTTP, no auth required for public topics.
- [ ] **`telegram_hook`** — Telegram Bot API `sendMessage`. Requires a bot token (not a webhook URL), so the client shape will differ slightly.

---

## hooksmith-core improvements

- [x] **Retry with exponential backoff** — add an optional `RetryPolicy` (max attempts, base delay, jitter) to `HttpClient`. Service crates can opt in by calling `post_json_with_retry`.
- [x] **Shared `WebhookError` base** — a common `CoreError` (network failure, JSON failure) that service-specific errors can wrap, so generic code can pattern-match on transport-level failures without knowing which service failed.
- [x] **`MockSender`** — a test-only `WebhookSender` implementation that captures sent messages in a `Vec`. Makes it easy to assert on notification content in unit tests without hitting any real endpoint.
- [x] **Tracing integration** — add optional `tracing` spans around every `post_json` call (request URL, status, latency) behind a `tracing` feature flag.
- [x] **`send_batch`** — a default method on `WebhookSender` that fans out a `Vec<&Message>` concurrently with `futures::join_all`.

---

## discord-hook improvements

- [ ] **File/attachment support** — multipart form upload so you can attach images, logs, or CSVs alongside a message.
- [ ] **Poll support** — Discord added native polls; expose the `poll` payload field.
- [ ] **Components V2** — action rows, buttons, select menus (these are send-only via webhook; you still need a bot to receive interaction callbacks).
- [ ] **Edit and delete** — `PATCH /webhooks/{id}/{token}/messages/{message_id}` and `DELETE` equivalents for editing or removing a previously sent message.
- [ ] **Forum channel thread creation** — `thread_name` and `applied_tags` are already serialized; add a dedicated builder method and docs for the forum channel workflow.
- [x] **Automatic rate-limit retry** — a `send_with_retry` convenience method that internally sleeps `retry_after_ms` and retries once on a 429.

---

## Developer experience

- [x] **Workspace-level examples** — an `examples/` folder at the repo root with runnable demos for each crate (requires only setting the relevant `*_WEBHOOK_URL` env var).
- [x] **`cargo test` integration tests** — tests that run against a real webhook URL when `DISCORD_WEBHOOK_URL` (etc.) are set in the environment, skipped otherwise.
- [x] **CI pipeline** — GitHub Actions: `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` on every PR.
- [ ] **Docs site** — `cargo doc --no-deps --open` works today; publishing to `docs.rs` happens automatically once the crate is on crates.io.
- [ ] **Publish all crates to crates.io** — `hooksmith-core` first, then service crates that depend on it.
- [x] **Changelog** (`CHANGELOG.md`) — track breaking changes and new features per release following [Keep a Changelog](https://keepachangelog.com).

---

## Security / hardening

- [x] **URL allowlist** — configurable set of trusted domains so `WebhookClient::new` rejects anything not in the list (useful for multi-tenant apps).
- [x] **Timeout configuration** — expose `connect_timeout` and `request_timeout` on the builders rather than relying on `reqwest`'s default (no timeout).
- [x] **Secrets scanning** — add a `.gitleaks.toml` or `trufflehog` config to catch accidentally committed webhook URLs in CI.
- [x] **Thread ID validation** — `send_to_thread` rejects non-numeric thread IDs at call time, preventing URL manipulation via injected characters.
- [x] **Response body size cap** — error response bodies are capped at 4 KiB to prevent a misbehaving endpoint from exhausting memory.
- [x] **Dependency CVE scanning** — `cargo audit` runs in CI against the RustSec advisory database.
