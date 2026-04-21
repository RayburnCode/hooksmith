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
}
