use std::future::Future;

/// Common interface implemented by every hooksmith webhook client.
///
/// Each service crate defines its own [`Message`](WebhookSender::Message) and
/// [`Error`](WebhookSender::Error) types and implements this trait.  Application
/// code can then be generic over the notification backend:
///
/// ```rust,ignore
/// use hooksmith_core::WebhookSender;
///
/// async fn notify<S>(sender: &S, msg: &S::Message) -> Result<(), S::Error>
/// where
///     S: WebhookSender,
/// {
///     sender.send(msg).await
/// }
/// ```
pub trait WebhookSender {
    /// The message type accepted by this sender.
    type Message;

    /// The error type returned on failure.
    type Error: std::error::Error;

    /// Send a single message to the configured webhook endpoint.
    fn send(
        &self,
        message: &Self::Message,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Send multiple messages concurrently, fanning them out with
    /// [`futures::future::join_all`].
    ///
    /// Returns one `Result` per message in the same order as the input slice.
    /// A failure for one message does **not** abort the others.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let results = sender.send_batch(&[&msg_a, &msg_b, &msg_c]).await;
    /// for result in results {
    ///     result?;
    /// }
    /// ```
    fn send_batch<'a>(
        &'a self,
        messages: &'a [&'a Self::Message],
    ) -> impl Future<Output = Vec<Result<(), Self::Error>>> + Send + 'a
    where
        Self: Sync,
        Self::Error: Send,
    {
        let futs: Vec<_> = messages.iter().copied().map(|m| self.send(m)).collect();
        async move { futures::future::join_all(futs).await }
    }
}
