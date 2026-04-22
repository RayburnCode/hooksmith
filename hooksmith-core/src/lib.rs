pub mod client;
pub mod error;
pub mod retry;
pub mod sender;

#[cfg(feature = "mock")]
pub mod mock;

pub use client::{is_allowed_domain, HttpClient, HttpClientBuilder};
pub use error::CoreError;
pub use retry::RetryPolicy;
pub use sender::WebhookSender;
