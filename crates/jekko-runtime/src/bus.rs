//! Async pub/sub event bus.
//!
//! Ported from `packages/jekko/src/bus/index.ts` and
//! `packages/jekko/src/bus/bus-event.ts`. Replaces the Effect/PubSub plumbing
//! with [`tokio::sync::broadcast`] for wildcard fan-out and per-type
//! subscriptions, plus [`tokio::sync::mpsc`] for ordered consumers.
//!
//! The TS bus publishes one of many strongly-typed `BusEvent.Definition`
//! payloads. In Rust we use [`serde_json::Value`] as the wire format and
//! tag events by a `type: &'static str` string — equivalent to the TS
//! schema-tagged union when read back over a JSON boundary.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::warn;

/// Lossy capacity for the wildcard broadcast channel.
const BROADCAST_CAPACITY: usize = 1024;

/// One published event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// Globally unique event id.
    pub id: String,
    /// Event type tag (e.g. `"session.status"`).
    #[serde(rename = "type")]
    pub kind: String,
    /// Free-form payload.
    pub properties: serde_json::Value,
}

/// A typed event definition. The runtime equivalent of the TS
/// `BusEvent.define("session.status", Schema.Struct(...))` factory.
#[derive(Clone, Debug)]
pub struct BusEvent {
    /// Event type tag.
    pub kind: &'static str,
}

impl BusEvent {
    /// Create a definition for a typed event with the given `type` tag.
    pub const fn new(kind: &'static str) -> Self {
        Self { kind }
    }
}

/// Async pub/sub bus.
#[derive(Debug)]
pub struct Bus {
    next_id: AtomicU64,
    wildcard: broadcast::Sender<EventEnvelope>,
    typed: RwLock<std::collections::HashMap<String, broadcast::Sender<EventEnvelope>>>,
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}

impl Bus {
    /// Construct an empty bus.
    pub fn new() -> Self {
        let (wildcard, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            next_id: AtomicU64::new(1),
            wildcard,
            typed: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Publish an event tagged with `kind` and the supplied JSON `properties`.
    ///
    /// Returns the generated event id. Lossy subscribers (no current
    /// listeners) silently drop the message — this mirrors the TS bus
    /// which only logs at debug.
    pub async fn publish(&self, kind: &str, properties: serde_json::Value) -> EventEnvelope {
        let id = self.next_id();
        let envelope = EventEnvelope {
            id,
            kind: kind.to_string(),
            properties,
        };

        // Fan out to wildcard (broadcast). Drop receivers silently.
        let _ = self.wildcard.send(envelope.clone());

        // Fan out to typed channel if present.
        let typed = self.typed.read().await;
        if let Some(sender) = typed.get(kind) {
            if let Err(err) = sender.send(envelope.clone()) {
                warn!(target: "bus", "no typed subscribers for {}: {}", kind, err);
            }
        }

        envelope
    }

    /// Subscribe to all events on the bus.
    pub fn subscribe_all(&self) -> broadcast::Receiver<EventEnvelope> {
        self.wildcard.subscribe()
    }

    /// Subscribe to events of a specific type. The subscription is lossy
    /// (broadcast); for guaranteed delivery use [`Bus::subscribe_mpsc`].
    pub async fn subscribe(&self, kind: &str) -> broadcast::Receiver<EventEnvelope> {
        let mut typed = self.typed.write().await;
        let sender = typed
            .entry(kind.to_string())
            .or_insert_with(|| broadcast::channel(BROADCAST_CAPACITY).0);
        sender.subscribe()
    }

    /// Subscribe to events of a specific type with a bounded mpsc channel.
    /// This guarantees order and back-pressure but at most one consumer.
    pub fn subscribe_mpsc(
        self: &Arc<Self>,
        kind: &'static str,
        buffer: usize,
    ) -> mpsc::Receiver<EventEnvelope> {
        let (tx, rx) = mpsc::channel(buffer);
        let bus = self.clone();
        tokio::spawn(async move {
            let mut sub = bus.subscribe(kind).await;
            while let Ok(envelope) = sub.recv().await {
                if tx.send(envelope).await.is_err() {
                    break;
                }
            }
        });
        rx
    }

    fn next_id(&self) -> String {
        let n = self.next_id.fetch_add(1, Ordering::SeqCst);
        format!("evt_{n:016x}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_and_subscribe() {
        let bus = Arc::new(Bus::new());
        let mut sub = bus.subscribe("test.event").await;
        let _ = bus
            .publish("test.event", serde_json::json!({ "n": 1 }))
            .await;
        let env = sub.recv().await.unwrap();
        assert_eq!(env.kind, "test.event");
        assert_eq!(env.properties["n"], 1);
    }

    #[tokio::test]
    async fn wildcard_receives_all_kinds() {
        let bus = Arc::new(Bus::new());
        let mut sub = bus.subscribe_all();
        let _ = bus.publish("a", serde_json::json!({})).await;
        let _ = bus.publish("b", serde_json::json!({})).await;
        let mut seen = Vec::new();
        for _ in 0..2 {
            seen.push(sub.recv().await.unwrap().kind);
        }
        seen.sort();
        assert_eq!(seen, vec!["a", "b"]);
    }

    #[tokio::test]
    async fn mpsc_consumes_in_order() {
        let bus = Arc::new(Bus::new());
        let mut rx = bus.subscribe_mpsc("ordered", 4);
        // give the spawned task a tick to subscribe
        tokio::task::yield_now().await;
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        for i in 0..3 {
            let _ = bus.publish("ordered", serde_json::json!({ "i": i })).await;
        }
        for i in 0..3 {
            let env = rx.recv().await.unwrap();
            assert_eq!(env.properties["i"], i);
        }
    }
}
