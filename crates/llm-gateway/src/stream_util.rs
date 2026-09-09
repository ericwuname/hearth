//! STREAM-1~4 (global-audit): abort-on-drop stream wrapper.
//!
//! All four streaming providers (openai / hunyuan / ollama / vllm) follow the
//! same pattern: `stream()` spawns a `tokio::spawn` task that reads the HTTP
//! byte stream and forwards parsed events through an mpsc channel, then drops
//! the `JoinHandle`. If the consumer drops the returned stream early (e.g.
//! session cancel), the spawned task keeps running until the next `tx.send`
//! fails — potentially holding the HTTP connection and burning tokens.
//!
//! `AbortOnDropStream` couples the consumer-side stream with the producer
//! task's `AbortHandle`: dropping the stream aborts the task immediately.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use tokio::task::AbortHandle;

/// A stream wrapper that aborts an associated tokio task when dropped.
///
/// Usage in a provider's `stream()`:
/// ```ignore
/// let (tx, rx) = futures::channel::mpsc::channel(64);
/// let handle = tokio::spawn(async move { /* produce into tx */ });
/// Box::pin(AbortOnDropStream::new(rx, handle.abort_handle()))
/// ```
pub struct AbortOnDropStream<S> {
    inner: S,
    abort: AbortHandle,
}

impl<S> AbortOnDropStream<S> {
    pub fn new(inner: S, abort: AbortHandle) -> Self {
        Self { inner, abort }
    }
}

impl<S> Drop for AbortOnDropStream<S> {
    fn drop(&mut self) {
        // Idempotent; aborting an already-finished task is a no-op.
        self.abort.abort();
    }
}

impl<S: Stream + Unpin> Stream for AbortOnDropStream<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    /// Dropping the wrapper must abort the producer task.
    #[tokio::test]
    async fn test_v12_abort_on_drop_kills_producer() {
        let finished_normally = Arc::new(AtomicBool::new(false));
        let flag = finished_normally.clone();

        let (mut tx, rx) = futures::channel::mpsc::channel::<u32>(4);
        let handle = tokio::spawn(async move {
            use futures::SinkExt;
            // Producer that would run for a long time unless aborted.
            for i in 0..10_000u32 {
                if tx.send(i).await.is_err() {
                    return;
                }
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
            flag.store(true, Ordering::SeqCst);
        });
        let abort_handle = handle.abort_handle();

        let mut stream = AbortOnDropStream::new(rx, abort_handle);

        // Consume one item, then drop the stream early.
        let first = stream.next().await;
        assert_eq!(first, Some(0));
        drop(stream);

        // The spawned task must finish as aborted (not run to completion).
        let join = handle.await;
        assert!(join.is_err(), "producer task should be aborted");
        assert!(join.unwrap_err().is_cancelled());
        assert!(!finished_normally.load(Ordering::SeqCst));
    }
}
