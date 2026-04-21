use reqwest::Client;
use serde::Serialize;

/// A thin wrapper around [`reqwest::Client`] shared by all hooksmith service crates.
///
/// Service crates (e.g. `discord_hook`, `slack_hook`) hold one of these,
/// configure it at construction time, and call [`HttpClient::post_json`] to
/// fire requests.
///
/// **TLS configuration** is the responsibility of the service crate — build a
/// [`reqwest::Client`] with your chosen TLS backend and pass it in via
/// [`HttpClient::with_reqwest`].
pub struct HttpClient {
    inner: Client,
}

impl HttpClient {
    /// Create a client backed by a freshly-constructed [`reqwest::Client`].
    ///
    /// A 30-second request timeout is applied by default so that a slow or
    /// unresponsive endpoint can never hang your application indefinitely.
    /// Override this by building your own [`reqwest::Client`] and passing it
    /// to [`HttpClient::with_reqwest`].
    pub fn new() -> Self {
        let inner = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to initialise reqwest client — TLS backend unavailable");
        Self { inner }
    }

    /// Wrap an existing [`reqwest::Client`].
    ///
    /// Use this to share a connection pool or inject custom configuration
    /// (timeouts, proxies, etc.) across your application.
    pub fn with_reqwest(client: Client) -> Self {
        Self { inner: client }
    }

    /// POST `body` serialized as JSON to `url` and return the raw response.
    pub async fn post_json(
        &self,
        url: &str,
        body: &impl Serialize,
    ) -> Result<reqwest::Response, reqwest::Error> {
        self.inner.post(url).json(body).send().await
    }

    /// Access the underlying [`reqwest::Client`] for advanced use-cases.
    pub fn inner(&self) -> &Client {
        &self.inner
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}
