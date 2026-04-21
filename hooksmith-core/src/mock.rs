use crate::WebhookSender;
use std::{future::Future, sync::Mutex};

/// A [`WebhookSender`] that records every sent message in memory instead of
/// dispatching to a real endpoint.
///
/// Enable the **`mock`** Cargo feature to use this in your crate's tests:
///
/// ```toml
/// [dev-dependencies]
/// hooksmith-core = { version = "*", features = ["mock"] }
/// ```
///
/// # Example
///
/// ```rust,ignore
/// use hooksmith_core::mock::MockSender;
/// use hooksmith_core::WebhookSender;
///
/// let sender: MockSender<MyMessage> = MockSender::new();
/// sender.send(&my_message).await.unwrap();
///
/// let captured = sender.messages();
/// assert_eq!(captured.len(), 1);
/// assert_eq!(captured[0], my_message);
/// ```
pub struct MockSender<M: Clone> {
    captured: Mutex<Vec<M>>,
}

impl<M: Clone> MockSender<M> {
    /// Create a new, empty [`MockSender`].
    pub fn new() -> Self {
        Self {
            captured: Mutex::new(Vec::new()),
        }
    }

    /// Return a clone of all messages captured so far, in send order.
    pub fn messages(&self) -> Vec<M> {
        self.captured.lock().unwrap().clone()
    }

    /// Return the number of messages captured so far.
    pub fn len(&self) -> usize {
        self.captured.lock().unwrap().len()
    }

    /// Returns `true` if no messages have been captured yet.
    pub fn is_empty(&self) -> bool {
        self.captured.lock().unwrap().is_empty()
    }

    /// Clear all captured messages.
    pub fn clear(&self) {
        self.captured.lock().unwrap().clear();
    }
}

impl<M: Clone> Default for MockSender<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// The infallible error type returned by [`MockSender`].
///
/// [`MockSender::send`] never actually fails; this type exists only to satisfy
/// the [`WebhookSender::Error`] associated-type bound.
#[derive(Debug)]
pub struct MockError;

impl std::fmt::Display for MockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mock sender error (this should never occur)")
    }
}

impl std::error::Error for MockError {}

impl<M: Clone + Send> WebhookSender for MockSender<M> {
    type Message = M;
    type Error = MockError;

    fn send(&self, message: &M) -> impl Future<Output = Result<(), MockError>> + Send {
        self.captured.lock().unwrap().push(message.clone());
        std::future::ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WebhookSender;

    #[tokio::test]
    async fn send_captures_message() {
        let sender: MockSender<String> = MockSender::new();
        assert!(sender.is_empty());

        sender.send(&"hello".to_string()).await.unwrap();

        assert_eq!(sender.len(), 1);
        assert_eq!(sender.messages(), vec!["hello".to_string()]);
    }

    #[tokio::test]
    async fn send_preserves_order() {
        let sender: MockSender<i32> = MockSender::new();
        for i in 0..5 {
            sender.send(&i).await.unwrap();
        }
        assert_eq!(sender.messages(), vec![0, 1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn clear_empties_captured_messages() {
        let sender: MockSender<i32> = MockSender::new();
        sender.send(&1).await.unwrap();
        sender.send(&2).await.unwrap();
        assert_eq!(sender.len(), 2);

        sender.clear();
        assert!(sender.is_empty());
        assert_eq!(sender.len(), 0);
    }

    #[tokio::test]
    async fn send_batch_captures_all_messages() {
        let sender: MockSender<String> = MockSender::new();
        let msgs = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let refs: Vec<&String> = msgs.iter().collect();

        let results = sender.send_batch(&refs).await;

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_ok()));
        assert_eq!(sender.len(), 3);
        assert_eq!(sender.messages(), msgs);
    }

    #[test]
    fn default_is_empty() {
        let sender: MockSender<()> = MockSender::default();
        assert!(sender.is_empty());
    }
}
