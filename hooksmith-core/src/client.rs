use crate::retry::RetryPolicy;
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;

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
            .timeout(Duration::from_secs(30))
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
    ///
    /// When the **`tracing`** feature is enabled this method emits an
    /// `info_span` named `hooksmith.post_json` capturing the request URL,
    /// HTTP status, and wall-clock latency.
    pub async fn post_json(
        &self,
        url: &str,
        body: &impl Serialize,
    ) -> Result<reqwest::Response, reqwest::Error> {
        #[cfg(not(feature = "tracing"))]
        {
            return self.inner.post(url).json(body).send().await;
        }

        #[cfg(feature = "tracing")]
        {
            use tracing::Instrument;
            let span = tracing::info_span!("hooksmith.post_json", url = %url);
            let start = std::time::Instant::now();
            let result = self
                .inner
                .post(url)
                .json(body)
                .send()
                .instrument(span.clone())
                .await;
            let latency_ms = start.elapsed().as_millis();
            let _enter = span.enter();
            match &result {
                Ok(resp) => tracing::info!(status = resp.status().as_u16(), latency_ms),
                Err(err) => tracing::error!(error = %err, latency_ms),
            }
            result
        }
    }

    /// POST `body` serialized as JSON to `url`, retrying on failure according
    /// to the supplied [`RetryPolicy`].
    ///
    /// Each retry is separated by an exponentially increasing delay
    /// (`base_delay × 2ⁿ`).  When `policy.jitter` is `true` a random
    /// fraction of the current step is added to the delay.
    ///
    /// Returns the first successful [`reqwest::Response`], or the error from
    /// the final attempt if all attempts are exhausted.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use hooksmith_core::RetryPolicy;
    ///
    /// let policy = RetryPolicy { max_attempts: 4, ..Default::default() };
    /// let resp = client.post_json_with_retry(url, &payload, &policy).await?;
    /// ```
    pub async fn post_json_with_retry(
        &self,
        url: &str,
        body: &impl Serialize,
        policy: &RetryPolicy,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let max = policy.max_attempts.max(1);
        let mut last_err: Option<reqwest::Error> = None;

        for attempt in 0..max {
            match self.post_json(url, body).await {
                Ok(resp) => return Ok(resp),
                Err(err) => {
                    let is_last = attempt + 1 >= max;
                    if !is_last {
                        let factor = 1u32 << attempt; // 2^attempt
                        let base = policy.base_delay * factor;
                        let delay = if policy.jitter {
                            // Use subsecond nanos from the system clock as a
                            // cheap jitter source — no external crate needed.
                            let nanos = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .subsec_nanos();
                            let jitter = (nanos % 1_000) as f64 / 1_000.0;
                            base + Duration::from_secs_f64(base.as_secs_f64() * jitter)
                        } else {
                            base
                        };
                        tokio::time::sleep(delay).await;
                    }
                    last_err = Some(err);
                }
            }
        }

        Err(last_err.expect("max_attempts is at least 1"))
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
