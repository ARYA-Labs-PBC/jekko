//! Question (free-form prompts to the user).
//!
//! Ported from `packages/jekko/src/question/index.ts`. Mirrors the
//! [`crate::permission`] ask flow but with free-form text answers.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;

use crate::bus::Bus;
use crate::error::{RuntimeError, RuntimeResult};

/// One question being asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionRequest {
    /// Question id.
    pub id: String,
    /// Owning session id.
    pub session_id: String,
    /// The question text.
    pub prompt: String,
    /// Optional choices the user can pick from.
    #[serde(default)]
    pub choices: Vec<String>,
}

/// Reply payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionReply {
    /// The free-form answer.
    pub answer: String,
}

/// Question service.
#[derive(Debug)]
pub struct QuestionService {
    bus: Arc<Bus>,
    inner: Mutex<HashMap<String, oneshot::Sender<QuestionReply>>>,
}

impl QuestionService {
    /// Construct.
    pub fn new(bus: Arc<Bus>) -> Self {
        Self {
            bus,
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Ask the user a question, returning the awaited reply.
    pub async fn ask(&self, mut req: QuestionRequest) -> RuntimeResult<QuestionReply> {
        if req.id.is_empty() {
            req.id = format!("question_{}", Uuid::new_v4().simple());
        }
        let (tx, rx) = oneshot::channel();
        self.inner.lock().await.insert(req.id.clone(), tx);
        let _ = self
            .bus
            .publish("question.asked", serde_json::to_value(&req)?)
            .await;
        rx.await.map_err(|_| RuntimeError::other("dropped"))
    }

    /// Reply to a pending question.
    pub async fn reply(&self, id: &str, reply: QuestionReply) -> RuntimeResult<()> {
        let tx = match self.inner.lock().await.remove(id) {
            Some(tx) => tx,
            None => return Err(RuntimeError::not_found("question", id)),
        };
        let _ = self
            .bus
            .publish(
                "question.replied",
                serde_json::json!({ "id": id, "answer": reply.answer }),
            )
            .await;
        let _ = tx.send(reply);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ask_reply_cycle() {
        let bus = Arc::new(Bus::new());
        let svc = Arc::new(QuestionService::new(bus.clone()));
        let mut sub = bus.subscribe("question.asked").await;

        let svc_clone = svc.clone();
        let h = tokio::spawn(async move {
            svc_clone
                .ask(QuestionRequest {
                    id: String::new(),
                    session_id: "s1".into(),
                    prompt: "ready?".into(),
                    choices: vec!["yes".into(), "no".into()],
                })
                .await
        });

        let envelope = sub.recv().await.unwrap();
        let id = envelope.properties["id"].as_str().unwrap().to_string();
        svc.reply(
            &id,
            QuestionReply {
                answer: "yes".into(),
            },
        )
        .await
        .unwrap();
        let reply = h.await.unwrap().unwrap();
        assert_eq!(reply.answer, "yes");
    }
}
